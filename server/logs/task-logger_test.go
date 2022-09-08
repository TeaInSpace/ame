package logs

import (
	"context"
	"fmt"
	"log"
	"os"
	"strings"
	"testing"
	"time"

	"golang.org/x/sync/errgroup"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/ameproject"
	"teainspace.com/ame/internal/auth"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/testcfg"
	testenv "teainspace.com/ame/internal/testenv"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	task "teainspace.com/ame/server/grpc"
)

var (
	testCfg           = testcfg.TestEnv()
	taskServiceClient task.TaskServiceClient
	restCfg           *rest.Config
	ctx               context.Context
)

func TestMain(m *testing.M) {
	testCfg = testcfg.TestEnv()
	ctx = context.Background()
	ctx = auth.AuthorarizeCtx(ctx, testCfg.AuthToken)

	var err error
	taskServiceClient, err = task.PrepareTaskClient(testCfg.AmeServerEndpoint)
	if err != nil {
		log.Fatal(err)
	}

	restCfg, err = clients.KubeClientFromConfig()
	if err != nil {
		log.Fatal(err)
	}

	os.Exit(m.Run())
}

func echoTask(ctx context.Context, messages []string) (*v1alpha1.Task, error) {
	p := ameproject.NewProjectForDir(testenv.EchoProjectDir, taskServiceClient)
	echoTask := v1alpha1.NewTask(fmt.Sprintf("python echo.py %s", strings.Join(messages, " ")), p.Name)
	return p.UploadAndRun(ctx, echoTask)
}

func matchLogs(t *v1alpha1.Task, expectedLogs []string) error {
	var actualLogs []TaskLogEntry
	streamCfg := StreamConfig{
		Follow:        true,
		StreamAllLogs: true,
		Task:          t,
		Sender: func(tle TaskLogEntry) error {
			actualLogs = append(actualLogs, tle)
			return nil
		},
		Timeout: time.Second * 60,
	}

	err := StreamTaskLogs(ctx, streamCfg, restCfg)
	if err != nil {
		return err
	}

	for _, expectedLogEntry := range expectedLogs {
		foundMatch := false
		for _, taskLogEntry := range actualLogs {
			if strings.Contains(string(taskLogEntry), expectedLogEntry) {
				foundMatch = true
				break
			}
		}

		if !foundMatch {
			return fmt.Errorf("task logs %v do no contain expected log entry: %s", actualLogs, expectedLogEntry)
		}
	}
	return nil
}

// TODO test for multiple task pods from the same project.

func TestCanStreamTaskLogs(t *testing.T) {
	testenv.SetupCluster(ctx, testCfg)

	// The logs are expected to contains these items not equal the list exactly as there will be a lot of output besides the output
	// from the run command.
	expectedLogs := []string{
		"logentryone",
		"logentrytwo",
	}

	echoTask, err := echoTask(ctx, expectedLogs)
	if err != nil {
		t.Fatal(err)
	}

	err = matchLogs(echoTask, expectedLogs)
	if err != nil {
		t.Error(err)
	}
}

func TestLogStreamFailsAfterTimeout(t *testing.T) {
	testenv.SetupCluster(ctx, testCfg)

	projectName := ameproject.ProjectNameFromDir(testenv.EchoProjectDir)

	streamCfg := StreamConfig{
		Follow:        true,
		StreamAllLogs: true,
		Task: &v1alpha1.Task{
			ObjectMeta: metav1.ObjectMeta{
				GenerateName: projectName,
				Namespace:    testCfg.Namespace,
			},
		},
		Sender: func(tle TaskLogEntry) error {
			return nil
		},
		Timeout: time.Second * 5,
	}

	timeStart := time.Now()
	err := StreamTaskLogs(ctx, streamCfg, restCfg)
	timeEnd := time.Now()

	timeoutSlack := time.Millisecond * 100

	// An error is to be expected as there is not actual task running to stream logs from.
	// This ensures that StreamTaskLogs will keep trying until the timeout duration has passed.
	if err == nil {
		t.Errorf("expected a non nil error from StreamTaskLogs, but got nil instread")
	}

	deadlineLower := timeStart.Add(streamCfg.Timeout - timeoutSlack)
	if timeEnd.Before(deadlineLower) {
		t.Errorf("expected StreamTaskLogs to run atleast within %v of duration: %v, but it only ran for duration: %v", timeoutSlack, streamCfg.Timeout, timeEnd.Sub(timeStart))
	}

	deadlineUpper := timeStart.Add(streamCfg.Timeout).Add(timeoutSlack)
	if timeEnd.After(deadlineUpper) {
		t.Errorf("expected StreamTaskLogs to stop within %v of the timeout duration: %v, but exceeded timeout by: %v", timeoutSlack, streamCfg.Timeout, timeEnd.Sub(timeStart.Add(streamCfg.Timeout)))
	}
}

// TODO: This test is flaky, we might be running out of memory, as both tasks will request 3GB of ram.
func TestCanStreamLogForMultipleTasksAtOnce(t *testing.T) {
	_, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	expectedLogsA := []string{
		"logsA1",
		"logsA2",
	}

	expectedLogsB := []string{
		"logsB1",
		"logsB2",
	}

	errGroup, ctx := errgroup.WithContext(ctx)
	testTask := func(expectedLogs []string) error {
		echoTask, err := echoTask(ctx, expectedLogs)
		if err != nil {
			t.Fatal(err)
		}

		return matchLogs(echoTask, expectedLogs)
	}

	errGroup.Go(func() error { return testTask(expectedLogsA) })
	errGroup.Go(func() error { return testTask(expectedLogsB) })

	err = errGroup.Wait()
	if err != nil {
		t.Error(err)
	}
}
