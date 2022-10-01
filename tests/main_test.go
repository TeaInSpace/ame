// The tests package contains tests which don't fit within a specific package and operate on a live cluster.
package tests

import (
	"context"
	"log"
	"os"
	"testing"
	"time"

	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/common"
	"teainspace.com/ame/internal/testcfg"
	testenv "teainspace.com/ame/internal/testenv"
)

var (
	ctx        context.Context
	testCfg    testcfg.TestEnvConfig
	taskClient common.AmeGenClient[*v1alpha1.Task]
)

func TestMain(m *testing.M) {
	ctx = context.Background()
	testCfg = testcfg.TestEnv()
	restCfg, err := clients.KubeClientFromConfig()
	if err != nil {
		log.Fatal(err)
	}

	taskClient = clients.GenericTaskClientFromConfig(restCfg, testCfg.Namespace)

	os.Exit(m.Run())
}

func TestGitSourceExecution(t *testing.T) {
	_, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	src := v1alpha1.NewGitSrc("https://github.com/jmintb/ame-showcase.git", "main")
	task := v1alpha1.NewTaskWithSrc("python main.py", "myproject", src)

	task, err = taskClient.Create(ctx, task)
	if err != nil {
		t.Fatal(err)
	}

	ctx, cancel := context.WithTimeout(ctx, time.Second*120)
	defer cancel()

	taskChan, err := taskClient.WatchObj(ctx, task.GetName())
	if err != nil {
		t.Fatal(err)
	}

	for task := range taskChan {
		if task.Status.Phase == v1alpha1.TaskSucceeded {
			return
		}

		if task.Status.Phase == v1alpha1.TaskFailed {
			t.Fatal("expected task to enter succeed, but phase: ", v1alpha1.TaskFailed)
		}
	}
}
