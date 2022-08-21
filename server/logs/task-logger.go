// Package logs implements utilities for fetch/streaming logs from a running Task.
package logs

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"time"

	v1 "k8s.io/api/core/v1"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/clients"
)

type (
	// A TaskLogEntry represents a single entry in the logs for a Task.
	TaskLogEntry string
	// A LogSender accepts a single TaskLogEntry.
	LogSender func(TaskLogEntry) error
)

// A StreamConfig determines how logs are streamed.
type StreamConfig struct {
	// Follow determines if the logs of the task should
	// continue to be streamed for the duration of the task.
	Follow bool

	// StreamAllLogs determinies if the log streaming should start from
	// the first log entry from the Task.
	StreamAllLogs bool

	// Task is the Task that logs will be streamed for.
	Task *amev1alpha1.Task

	// Sender is the LogSender that will processes each TaskLogEntry.
	Sender LogSender

	// Timeout determines the duration StreamTaskLogs is allowed to wait for Task
	// to start logging, once the Logging has commenced Timeout no longer has any
	// effect.
	Timeout time.Duration
}

// StreamTaskLogs streams the logs for the pod executing the task specified in streamCfg.
// The Sender function supplied in streamCfg is called for each log entry, any error returned by Sender is returned by StreamTaskLogs.
//
// The timeout in streamCfg sets a deadline for how long the function will wait for the task to start streaming logs.
// Once the log streaming has commenced, the timeout has no effect.
func StreamTaskLogs(ctx context.Context, streamCfg StreamConfig, restCfg *rest.Config) error {
	ctxWithTimout, cancelCtx := context.WithTimeout(ctx, streamCfg.Timeout)
	defer cancelCtx()
	pods := clients.PodClientFromConfig(restCfg, streamCfg.Task.GetNamespace())
	taskPod, err := amev1alpha1.GetTaskPod(ctxWithTimout, pods, streamCfg.Task)
	if err != nil {
		return err
	}

	// Note that main container is selected here for logging, as that is the container where
	// Argo runs the workflow within the Pod.
	req := pods.GetLogs(taskPod.GetName(), &v1.PodLogOptions{
		Container:  "main",
		Follow:     streamCfg.Follow,
		SinceTime:  &taskPod.CreationTimestamp,
		Timestamps: true,
	})

	// TODO: How can we test that this loop times out correctly?
	// TODO we should wait for the pod to finish initializing before requesting a stream.
	var reader io.ReadCloser
	for {
		select {
		case <-ctxWithTimout.Done():
			return fmt.Errorf("failed to get logs for main container within timeout %v, with error: %v", streamCfg.Timeout, err)
		default:
			reader, err = req.Stream(ctx)
		}

		if err == nil {
			break
		}

		time.Sleep(time.Millisecond * 50)
	}

	// At this point the work with a deadline is completed, so
	// the context with a timeout can be cancelled.
	cancelCtx()

	// TODO: Test for handling cancelled context.
	scanner := bufio.NewScanner(reader)
	for {
		taskPod, err = pods.Get(ctx, taskPod.GetName(), metav1.GetOptions{})
		if err != nil {
			return err
		}

		newContent := scanner.Scan()
		if !newContent {
			return nil
		}

		err = streamCfg.Sender(TaskLogEntry(scanner.Bytes()))
		if err != nil {
			return err
		}

		if taskPod.Status.Phase != v1.PodRunning {
			return nil
		}
	}
}
