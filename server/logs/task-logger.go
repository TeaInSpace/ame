// Package logs implements utilities for fetch/streaming logs from a running Task.
package logs

import (
	"bufio"
	"context"
	"time"

	"golang.org/x/sync/errgroup"
	v1 "k8s.io/api/core/v1"
	v1client "k8s.io/client-go/kubernetes/typed/core/v1"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/common"
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
	genPodClient := common.NewAmeGenClient[*v1.Pod](pods)

	taskCli := clients.GenericTaskClientFromConfig(restCfg, streamCfg.Task.GetNamespace())
	err := taskCli.WaitForCond(ctxWithTimout, streamCfg.Task.GetName(), common.IsTaskRunning)
	if err != nil {
		return err
	}

	// At this point the work with a deadline is completed, so
	// the context with a timeout can be cancelled.
	cancelCtx()

	logChan := make(chan TaskLogEntry)
	defer close(logChan)

	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	listOptions := metav1.ListOptions{
		Watch:         true,
		LabelSelector: "ame-task=" + streamCfg.Task.GetName(),
		FieldSelector: "status.phase=" + string(v1.PodRunning),
	}
	podChan, err := genPodClient.Watch(ctx, listOptions)
	if err != nil {
		return err
	}

	eg, grpCtx := errgroup.WithContext(ctx)
	go processPods(podChan, func(p *v1.Pod) {
		logOpts := &v1.PodLogOptions{
			Container:  "main",
			Follow:     streamCfg.Follow,
			SinceTime:  &p.CreationTimestamp,
			Timestamps: false,
		}

		eg.Go(func() error {
			return logPod(grpCtx, p, pods, logChan, logOpts)
		})
	})

	eg.Go(func() error {
		for {
			select {
			case <-ctx.Done():
				return nil
			case l := <-logChan:
				err := streamCfg.Sender(l)
				if err != nil {
					return err
				}
			}
		}
	})

	taskChan, err := taskCli.WatchObj(ctx, streamCfg.Task.GetName())
	if err != nil {
		return err
	}

	for t := range taskChan {
		if t.Status.Phase != amev1alpha1.TaskRunning {
			cancel()
			eg.Wait()
			break
		}
	}

	return nil
}

func processPods(podChan <-chan *v1.Pod, apply func(*v1.Pod)) {
	seenPods := make(map[string]bool)

	for pod := range podChan {
		if seenPods[pod.GetName()] {
			continue
		}

		seenPods[pod.GetName()] = true
		apply(pod)
	}
}

func logPod(ctx context.Context, p *v1.Pod, pods v1client.PodInterface, logChan chan<- TaskLogEntry, logOpts *v1.PodLogOptions) error {
	// Note that main container is selected here for logging, as that is the container where
	// Argo runs the workflow within the Pod.
	req := pods.GetLogs(p.GetName(), logOpts)

	// TODO: How can we test that this loop times out correctly?
	// TODO we should wait for the pod to finish initializing before requesting a stream.
	reader, err := req.Stream(ctx)
	if err != nil {
		return err
	}
	defer reader.Close()

	listOpts, err := common.GenNameListOpts(p.GetName())
	if err != nil {
		return err
	}

	watcher, err := pods.Watch(ctx, listOpts)
	if err != nil {
		// TODO handle error
		return err
	}

	scanner := bufio.NewScanner(reader)
	for {
		select {
		case <-ctx.Done():
			return nil
		case e := <-watcher.ResultChan():
			pod, ok := e.Object.(*v1.Pod)
			if ok && pod.Status.Phase != v1.PodRunning {
				return nil
			}
		default:
			ok := scanner.Scan()
			err = scanner.Err()
			if err != nil {
				return err
			}
			if ok {
				logChan <- TaskLogEntry(scanner.Bytes())
			}

		}
	}
}
