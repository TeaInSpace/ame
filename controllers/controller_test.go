package controllers

import (
	"context"
	"fmt"
	"strings"
	"testing"
	"time"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	"github.com/brianvoe/gofakeit/v6"
	"github.com/google/go-cmp/cmp"
	. "github.com/onsi/ginkgo"
	. "github.com/onsi/gomega"
	"github.com/onsi/gomega/gstruct"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"teainspace.com/ame/api/v1alpha1"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/common"
	"teainspace.com/ame/internal/workflows"
)

// TODO: test different namespaces.
var testNamespace = "default"

func genTask(name string, namespace string) amev1alpha1.Task {
	return amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{Name: name, Namespace: namespace},
		TypeMeta:   v1.TypeMeta{APIVersion: "ame.teainspace.com/v1alpha1", Kind: "Task"},
		Spec: amev1alpha1.TaskSpec{
			RunCommand: "python train.py",
			ProjectId:  "myprojectid",
		},
	}
}

func getParameterByName(parameters []argo.Parameter, name string) argo.Parameter {
	for _, p := range parameters {
		if p.Name == name {
			return p
		}
	}

	return argo.Parameter{}
}

var _ = Describe("Task execution", func() {
	AfterEach(func() {
		err := k8sClient.DeleteAllOf(ctx, &argo.Workflow{}, &client.DeleteAllOfOptions{ListOptions: client.ListOptions{Namespace: testNamespace}})
		Expect(err).ToNot(HaveOccurred())

		err = k8sClient.DeleteAllOf(ctx, &amev1alpha1.Task{}, &client.DeleteAllOfOptions{ListOptions: client.ListOptions{Namespace: testNamespace}})
		Expect(err).ToNot(HaveOccurred())
	})

	It("Can create an argo workflow to execute a task", func() {
		// TODO: should we context.Background?
		ctx := context.Background()

		test := genTask(strings.ToLower(gofakeit.Username()), testNamespace)

		// Ensure that a Workflow for the Task does not already exist
		// before creating it.
		err := GetArgoWorkflow(ctx, k8sClient, test, &argo.Workflow{})
		Expect(err).To(MatchError(NewWorkflowNotFoundError(test)))

		err = k8sClient.Create(ctx, &test)
		Expect(err).ToNot(HaveOccurred())

		Eventually(func() (argo.Workflow, error) {
			wf := argo.Workflow{}
			err := GetArgoWorkflow(ctx, k8sClient, test, &wf)
			return wf, err
		}, "500ms").Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
			"ObjectMeta": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"Namespace": Equal(test.Namespace),
				"Name":      ContainSubstring(test.GetName()),
			}),
		}))
	})

	It("Does recreate an argo workflow if the existing workflow is deleted", func() {
		ctx := context.Background()
		test := genTask(strings.ReplaceAll(gofakeit.Noun(), " ", ""), testNamespace)

		err := k8sClient.Create(ctx, &test)
		Expect(err).ToNot(HaveOccurred())

		// Ensure that the workflow exists before deleting it.
		expectedWorkflow := argo.Workflow{}
		Eventually(func() error {
			err = GetArgoWorkflow(ctx, k8sClient, test, &expectedWorkflow)
			return err
		}, "100ms").Should(Not(HaveOccurred()))

		err = k8sClient.Delete(ctx, &expectedWorkflow)
		Expect(err).ToNot(HaveOccurred())

		// Before verfiying that the UIDs are not equal, it is important to
		// check that the initial Workflow's UID is not empty. As that would
		// make the comparison meaningless.
		Expect(expectedWorkflow.UID).ToNot(BeEmpty())
		Eventually(func() (argo.Workflow, error) {
			wf := argo.Workflow{}
			err = GetArgoWorkflow(ctx, k8sClient, test, &wf)
			return wf, err
		}, "1s").Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
			"ObjectMeta": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"UID": Not(Equal(expectedWorkflow.UID)),
			}),
		}))
	})

	// TODO: should we test that workflows are not recreated if one already exists for a task?
	// TODO: should we test that workflows are deleted when a task is deleted?
})

func createTestTask(ctx context.Context) (*amev1alpha1.Task, error) {
	t := amev1alpha1.NewTask("python train.py", "myproject")
	return tasks.Create(ctx, t, v1.CreateOptions{})
}

func TestCorrectsMisconfiguredWf(t *testing.T) {
	task, err := createTestTask(ctx)
	if err != nil {
		t.Fatal(err)
	}

	originalWf, err := workflows.WaitForTaskWorkflow(ctx, workflowClient, task.GetName(), time.Second)
	if err != nil {
		t.Fatal(err)
	}

	badWf := originalWf.DeepCopy()
	badWf.Spec.Templates[0].Steps[0].Steps[0].Inline.Script.Source = "bad script"
	_, err = workflowClient.Update(ctx, badWf, v1.UpdateOptions{})
	if err != nil {
		t.Fatal(err)
	}

	timeOut := time.Millisecond * 500
	time.Sleep(timeOut)

	var correctedWf argo.Workflow
	err = GetArgoWorkflow(ctx, k8sClient, *task, &correctedWf)
	if err != nil {
		t.Fatal(err)
	}

	// Checking the UID ensures that controller has patched the existing object.
	if correctedWf.GetUID() != originalWf.GetUID() {
		t.Errorf("expected UID to be idental for corrected object, but %s!=%s", correctedWf.GetUID(), originalWf.GetUID())
	}

	diff := cmp.Diff(correctedWf.Spec, originalWf.Spec)
	if diff != "" {
		t.Errorf("expected correctedWf=cronWf, but got diff: %s", diff)
	}
}

func TestTaskPhaseMatchesWorkflowPhase(t *testing.T) {
	testTask, err := createTestTask(ctx)
	if err != nil {
		t.Fatal(err)
	}

	wf, err := workflows.WaitForTaskWorkflow(ctx, workflowClient, testTask.GetName(), time.Second)
	if err != nil {
		t.Fatal(err)
	}

	testTask, err = taskGenClient.Cli.Get(ctx, testTask.GetName(), v1.GetOptions{})
	if err != nil {
		t.Fatal(err)
	}

	tcs := []struct {
		taskPhase     v1alpha1.TaskPhase
		workflowPhase argo.WorkflowPhase
	}{
		{
			taskPhase:     v1alpha1.TaskRunning,
			workflowPhase: argo.WorkflowRunning,
		},
		{
			taskPhase:     v1alpha1.TaskPending,
			workflowPhase: argo.WorkflowPending,
		},
		{
			taskPhase:     v1alpha1.TaskFailed,
			workflowPhase: argo.WorkflowFailed,
		},
		{
			taskPhase:     v1alpha1.TaskFailed,
			workflowPhase: argo.WorkflowError,
		},
		{
			taskPhase:     v1alpha1.TaskSucceeded,
			workflowPhase: argo.WorkflowSucceeded,
		},
	}

	// TODO: The test cases currently interefere with each other, as the state is not reset.
	// TODO: Use events to verify that the object was actually chanaged.
	for _, tc := range tcs {
		t.Run(fmt.Sprintf("taskPhase: %s, workflowPhase: %s", tc.taskPhase, tc.workflowPhase), func(t *testing.T) {
			wf.Status.Phase = tc.workflowPhase
			wf, err = workflowGenClient.Cli.Update(ctx, wf, v1.UpdateOptions{})
			if err != nil {
				t.Fatal(err)
			}

			err = taskGenClient.WaitForCondWithTimout(ctx, testTask.GetName(), common.GenTaskPhaseValidator(tc.taskPhase), time.Second)
			if err != nil {
				t.Error(err)
			}
		})
	}
}

func TestHandleGitSourceCorrectly(t *testing.T) {
	source := v1alpha1.NewGitSrc("myrepo", "myref")
	task := v1alpha1.NewTaskWithSrc("mycmd", "myproject", source)
	task, err := tasks.Create(ctx, task, v1.CreateOptions{})
	if err != nil {
		t.Fatal(err)
	}

	wf, err := workflows.WaitForTaskWorkflow(ctx, workflowClient, task.GetName(), time.Second)
	if err != nil {
		t.Fatal(err)
	}

	wfSetupScript := wf.Spec.Templates[0].Steps[0].Steps[0].Inline.Script.Source
	expectedCloneCmd := "git clone " + source.GitRepository
	expectedCheckoutCmd := "git checkout " + source.GitReference

	if !strings.Contains(wfSetupScript, expectedCloneCmd) {
		t.Errorf("expected setup script to contain %s, but got %s instead", expectedCloneCmd, wfSetupScript)
	}

	if !strings.Contains(wfSetupScript, expectedCheckoutCmd) {
		t.Errorf("expected setup script to contain %s, but got %s instead", expectedCheckoutCmd, wfSetupScript)
	}
}
