package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"testing"

	argo "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"

	"github.com/stretchr/testify/assert"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/tools/clientcmd"

	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	clientset "teainspace.com/ame/generated/clientset/versioned"
	"teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/server/storage"
)

const (
	testNamespace   = "ame-system"
	testDataDir     = "test_data"
	testProjectName = "ame"
	testBucketName  = "mybucket"
)

var (
	tasks          v1alpha1.TaskInterface
	workflows      argo.WorkflowInterface
	ctx            context.Context
	testProjectDir string
)

func TestMain(m *testing.M) {
	ctx = context.Background()
	configLoadingRules := clientcmd.NewDefaultClientConfigLoadingRules()
	configOverrides := &clientcmd.ConfigOverrides{}
	kubeConfig := clientcmd.NewNonInteractiveDeferredLoadingClientConfig(configLoadingRules, configOverrides)
	config, err := kubeConfig.ClientConfig()
	if err != nil {
		log.Fatal(err)
	}

	argoClientSent := argo.NewForConfigOrDie(config)
	workflows = argoClientSent.Workflows(testNamespace)

	cliSet := clientset.NewForConfigOrDie(config)
	tasks = cliSet.AmeV1alpha1().Tasks(testNamespace)

	taskList, err := tasks.List(ctx, v1.ListOptions{})
	if err != nil {
		log.Fatal(err)
	}

	for _, ta := range taskList.Items {
		err := tasks.Delete(ctx, ta.GetName(), v1.DeleteOptions{})
		if err != nil {
			log.Fatal(err)
		}
	}

	path, err := os.Getwd()
	if err != nil {
		log.Fatal(err)
	}

	testProjectDir = fmt.Sprintf("%s/%s/%s", path, testDataDir, testProjectName)

	exitCode := m.Run()
	err = os.RemoveAll(testDataDir)
	if err != nil {
		log.Fatal(err)
	}
	os.Exit(exitCode)
}

func TestRun(t *testing.T) {
	err := os.Chdir(testProjectDir)
	assert.NoError(t, err)
	files := []storage.ProjectFile{
		{
			Path: "somefile.txt",
			Data: []byte("somecontents"),
		},
	}

	dirtools.PopulateDir(testProjectDir, files)

	testTask := amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{
			Name: testProjectName,
		}, Spec: amev1alpha1.TaskSpec{
			RunCommand: "python test.py",
		},
	}

	cmd := exec.Command("go", "run", "../../main.go", "run", testTask.Spec.RunCommand)
	cmd.Dir = testProjectDir
	out, err := cmd.CombinedOutput()
	assert.NoError(t, err)

	fmt.Println(string(out))
	inclusterTask, err := tasks.Get(ctx, testProjectName, v1.GetOptions{})
	assert.NoError(t, err)
	assert.Equal(t, testTask.Spec.RunCommand, inclusterTask.Spec.RunCommand)
	assert.Contains(t, string(out), "Your task will be executed!")

	wfList, err := workflows.List(ctx, v1.ListOptions{})
	assert.NoError(t, err)
	assert.Equal(t, testTask.Spec.RunCommand, wfList.Items[0].Spec.Arguments.Parameters[1].Value.String())

	s3Client, err := storage.CreateS3ClientForLocalStorage(ctx)
	assert.NoError(t, err)
	storage := storage.NewS3Storage(*s3Client, testBucketName)
	storedFiles, err := storage.DownloadFiles(ctx, testProjectName)
	assert.ElementsMatch(t, files, storedFiles)

	err = os.Chdir("../../")
	assert.NoError(t, err)
}
