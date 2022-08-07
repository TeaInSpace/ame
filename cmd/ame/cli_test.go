package main

import (
	"context"
	"log"
	"os"
	"os/exec"
	"path"
	"strings"
	"testing"

	argo "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"

	"github.com/stretchr/testify/assert"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/tools/clientcmd"

	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/server/storage"
)

const (
	testNamespace  = "ame-system"
	testBucketName = "ameprojectstorage"
)

var (
	tasks     v1alpha1.TaskInterface
	workflows argo.WorkflowInterface
	ctx       context.Context
)

func kubeClientFromConfig() (*rest.Config, error) {
	configLoadingRules := clientcmd.NewDefaultClientConfigLoadingRules()
	configOverrides := &clientcmd.ConfigOverrides{}
	kubeConfig := clientcmd.NewNonInteractiveDeferredLoadingClientConfig(configLoadingRules, configOverrides)
	config, err := kubeConfig.ClientConfig()
	if err != nil {
		return nil, err
	}

	return config, nil
}

func workflowsClientFromConfig(cfg *rest.Config, ns string) argo.WorkflowInterface {
	return argo.NewForConfigOrDie(cfg).Workflows(ns)
}

func tasksClientFromConfig(cfg *rest.Config, ns string) v1alpha1.TaskInterface {
	return v1alpha1.NewForConfigOrDie(cfg).Tasks(ns)
}

func clearTasksInCluster() error {
	taskList, err := tasks.List(ctx, v1.ListOptions{})
	if err != nil {
		return err
	}

	for _, ta := range taskList.Items {
		err := tasks.Delete(ctx, ta.GetName(), v1.DeleteOptions{})
		if err != nil {
			return err
		}
	}

	return nil
}

func TestMain(m *testing.M) {
	ctx = context.Background()
	kubeCfg, err := kubeClientFromConfig()
	if err != nil {
		log.Fatal(err)
	}

	workflows = workflowsClientFromConfig(kubeCfg, testNamespace)
	tasks = tasksClientFromConfig(kubeCfg, testNamespace)

	err = clearTasksInCluster()
	if err != nil {
		log.Fatal(err)
	}

	os.Exit(m.Run())
}

func setupObjectStorage(s storage.Storage) error {
	err := s.ClearStorage(ctx)
	if err != nil {
		return err
	}
	err = s.PrepareStorage(ctx)
	if err != nil {
		return err
	}

	return nil
}

func TestRun(t *testing.T) {
	s3Client, err := storage.CreateS3ClientForLocalStorage(ctx)
	assert.NoError(t, err)
	store := storage.NewS3Storage(*s3Client, testBucketName)
	err = setupObjectStorage(store)
	assert.NoError(t, err)

	files := []storage.ProjectFile{
		{
			Path: "somefile.txt",
			Data: []byte("somecontents"),
		},
	}

	testDir, err := dirtools.MkDirTempAndApply("myproject", dirtools.ApplyFilesToDir(files))
	assert.NoError(t, err)

	projectName := path.Base(testDir)

	testTask := amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{
			Name: projectName,
		}, Spec: amev1alpha1.TaskSpec{
			RunCommand: "python test.py",
		},
	}

	wd, err := os.Getwd()
	assert.NoError(t, err)
	cmd := exec.Command("go", "run", path.Join(wd, "main.go"), "run", testTask.Spec.RunCommand)
	cmd.Dir = testDir
	out, err := cmd.CombinedOutput()
	assert.NoError(t, err)

	inclusterTask, err := tasks.Get(ctx, projectName, v1.GetOptions{})
	assert.NoError(t, err)
	assert.Equal(t, testTask.Spec.RunCommand, inclusterTask.Spec.RunCommand)
	assert.Contains(t, string(out), "Your task will be executed!")

	wfList, err := workflows.List(ctx, v1.ListOptions{})
	assert.NoError(t, err)
	assert.Equal(t, testTask.Spec.RunCommand, wfList.Items[0].Spec.Arguments.Parameters[1].Value.String())

	storedFiles, err := store.DownloadFiles(ctx, projectName)
	assert.NoError(t, err)

	trimmedStored := []storage.ProjectFile{}
	for _, f := range storedFiles {
		p := strings.Replace(f.Path, projectName+"/", "", 1)
		trimmedStored = append(trimmedStored, storage.ProjectFile{
			Path: p,
			Data: f.Data,
		})
	}

	assert.ElementsMatch(t, files, trimmedStored)

	err = store.ClearStorage(ctx)
	assert.NoError(t, err)
}
