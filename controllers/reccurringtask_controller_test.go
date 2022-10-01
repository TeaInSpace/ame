package controllers

import (
	"context"
	"log"
	"os"
	"path/filepath"
	"testing"
	"time"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	argoClients "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
	"github.com/google/go-cmp/cmp"
	"golang.org/x/sync/errgroup"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/envtest"
	"teainspace.com/ame/api/v1alpha1"
	ameClients "teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/common"
	"teainspace.com/ame/internal/workflows"
)

var (
	envTest           *envtest.Environment
	recTasks          ameClients.ReccurringTaskInterface
	tasks             ameClients.TaskInterface
	cronWorkflows     argoClients.CronWorkflowInterface
	workflowClient    argoClients.WorkflowInterface
	k8sManager        ctrl.Manager
	taskGenClient     common.AmeGenClient[*v1alpha1.Task]
	workflowGenClient common.AmeGenClient[*argo.Workflow]
)

func TestMain(m *testing.M) {
	ctx, cancel = context.WithCancel(context.Background())

	envTest = &envtest.Environment{
		CRDDirectoryPaths:     []string{filepath.Join("..", "config", "crd", "bases")},
		ErrorIfCRDPathMissing: true,
	}
	defer envTest.Stop()

	var err error
	cfg, err = envTest.Start()
	if err != nil {
		log.Fatalln(err)
	}

	k8sClient, err = client.New(cfg, client.Options{
		Scheme: scheme.Scheme,
	})
	if err != nil {
		log.Fatalln(err)
	}

	err = v1alpha1.AddToScheme(scheme.Scheme)
	if err != nil {
		log.Fatalln(err)
	}

	err = argo.AddToScheme(scheme.Scheme)
	if err != nil {
		log.Fatalln(err)
	}

	k8sManager, err = ctrl.NewManager(cfg, ctrl.Options{
		Scheme: scheme.Scheme,
	})
	if err != nil {
		log.Fatalln(err)
	}

	recTasks = clients.RecTasksClientFromConfig(cfg, testNamespace)
	tasks = clients.TasksClientFromConfig(cfg, testNamespace)
	taskGenClient = common.NewAmeGenClient[*v1alpha1.Task](tasks)
	cronWorkflows = clients.CronWorkflowsClientFromConfig(cfg, testNamespace)
	workflowClient = clients.WorkflowsClientFromConfig(cfg, testNamespace)
	workflowGenClient = common.NewAmeGenClient[*argo.Workflow](workflowClient)

	err = (&ReccurringTaskReconciler{
		k8sClient,
		scheme.Scheme,
		cronWorkflows,
	}).SetupWithManager(k8sManager)
	if err != nil {
		log.Fatalln(err)
	}

	err = (&TaskReconciler{
		k8sClient, scheme.Scheme,
	}).SetupWithManager(k8sManager)
	if err != nil {
		log.Fatal(err)
	}

	errGrp, ctx := errgroup.WithContext(ctx)
	errGrp.Go(func() error {
		return k8sManager.Start(ctx)
	})

	exitCode := m.Run()

	cancel()
	err = errGrp.Wait()
	if err != nil {
		log.Fatal(err)
	}

	os.Exit(exitCode)
}

func createTestRecTask(ctx context.Context) (*v1alpha1.ReccurringTask, error) {
	taskSpec := v1alpha1.TaskSpec{
		RunCommand: "pytho train.py",
		ProjectId:  "myproject",
	}

	recTask := v1alpha1.NewRecurringTask("myrectask", taskSpec, "0 0 12 0")
	return recTasks.Create(ctx, recTask.DeepCopy(), v1.CreateOptions{})
}

func TestCreatesCronWorkflow(t *testing.T) {
	cwfs, err := cronWorkflows.List(ctx, v1.ListOptions{})
	if err != nil {
		t.Fatal(err)
	}

	if len(cwfs.Items) != 0 {
		t.Errorf("expected len(cwfs)=0, but got len(cwfs)=%d", len(cwfs.Items))
	}

	recTask, err := createTestRecTask(ctx)
	if err != nil {
		t.Fatal(err)
	}

	timeOut := time.Millisecond * 500
	cronWf, err := workflows.WaitForCronWfForRecTask(ctx, cronWorkflows, recTask.GetName(), timeOut)
	if err != nil {
		t.Fatalf("failed to created CronWorkflow within timeout: %v, got error:%v ", timeOut, err)
	}

	correctCronWf, err := workflows.GenCronWf(recTask, scheme.Scheme)
	if err != nil {
		t.Fatal(err)
	}

	diff := cmp.Diff(cronWf.Spec, correctCronWf.Spec)
	if diff != "" {
		t.Errorf("expected a correct CronWorflowSpec, but got diff: %s", diff)
	}
}

func TestCorrectsMisconfiguredCronWf(t *testing.T) {
	recTask, err := createTestRecTask(ctx)
	if err != nil {
		t.Fatal(err)
	}

	timeOut := time.Millisecond * 500

	originCronWf, err := workflows.WaitForCronWfForRecTask(ctx, cronWorkflows, recTask.GetName(), timeOut)
	if err != nil {
		t.Fatal(err)
	}

	badCronWf := originCronWf.DeepCopy()
	badCronWf.Spec.Schedule = "sdfdfd"

	_, err = cronWorkflows.Update(ctx, badCronWf, v1.UpdateOptions{})
	if err != nil {
		t.Fatal(err)
	}

	time.Sleep(timeOut)

	correctEdCronWf, err := workflows.CronWfForRecTask(ctx, cronWorkflows, recTask.GetName())
	if err != nil {
		t.Fatal(err)
	}

	// Checking the UID ensures that controller has patched the existing object.
	if correctEdCronWf.GetUID() != originCronWf.GetUID() {
		t.Errorf("expected UID to be idental for corrected object, but %s!=%s", correctEdCronWf.GetUID(), originCronWf.GetUID())
	}

	diff := cmp.Diff(correctEdCronWf.Spec, originCronWf.Spec)
	if diff != "" {
		t.Errorf("expected correctedCronWf=cronWf, but got diff: %s", diff)
	}
}

func TestRecreateCronWfOnDeletion(t *testing.T) {
	recTask, err := createTestRecTask(ctx)
	if err != nil {
		t.Error(err)
	}

	timeOut := time.Millisecond * 500
	cronWf, err := workflows.WaitForCronWfForRecTask(ctx, cronWorkflows, recTask.GetName(), timeOut)
	if err != nil {
		t.Fatal(err)
	}

	err = cronWorkflows.Delete(ctx, cronWf.GetName(), v1.DeleteOptions{})
	if err != nil {
		t.Fatal(err)
	}

	_, err = workflows.WaitForCronWfForRecTask(ctx, cronWorkflows, recTask.GetName(), timeOut)
	if err != nil {
		t.Errorf("failed to recreated timeout after deletion within timeout: %v", timeOut)
	}
}
