// The tests package contains tests which don't fit within a specific package and operate on a live cluster.
package tests

import (
	"context"
	"log"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/google/go-cmp/cmp"
	v1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
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
	podClient  common.AmeGenClient[*v1.Pod]
)

func TestMain(m *testing.M) {
	ctx = context.Background()
	testCfg = testcfg.TestEnv()
	restCfg, err := clients.KubeClientFromConfig()
	if err != nil {
		log.Fatal(err)
	}

	taskClient = clients.GenericTaskClientFromConfig(restCfg, testCfg.Namespace)
	podClient = clients.GenericPodClientFromConfig(restCfg, testCfg.Namespace)
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

func TestCanGenerateWorkflowWithResources(t *testing.T) {
	_, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	task := v1alpha1.NewTask("test", "myproject")
	task.Spec.Resources = v1.ResourceList{
		"memory":         resource.MustParse("8"),
		"cpu":            resource.MustParse("4"),
		"nvidia.com/gpu": resource.MustParse("1"),
	}

	task, err = taskClient.Create(ctx, task)
	if err != nil {
		t.Fatal(err)
	}

	ctx, cancel := context.WithTimeout(ctx, time.Second*120)
	defer cancel()

	podChan, err := podClient.Watch(ctx, metav1.ListOptions{
		LabelSelector: "ame-task=" + task.GetName(),
	})
	if err != nil {
		t.Fatal(err)
	}

	for p := range podChan {
		// We are only interested in the pod for the main, as that is where the pod spec will be applied.
		if strings.Contains(p.GetName(), "setup") {
			continue
		}

		for _, c := range p.Spec.Containers {
			if c.Name == "main" {
				diff := cmp.Diff(c.Resources.Limits, task.Spec.Resources)
				if diff != "" {
					t.Errorf("expected pod resource limits to match task resources but got diff:\n%s", diff)
				}
				return
			}
		}
	}

	t.Fatal("could not find a valid container")
}
