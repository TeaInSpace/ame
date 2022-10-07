package common

import (
	"context"
	"fmt"
	"time"

	v1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/fields"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/watch"
	clientv1 "k8s.io/client-go/kubernetes/typed/core/v1"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/workflows"
)

type AmeKubeClient[obj any] interface {
	Get(context.Context, string, metav1.GetOptions) (obj, error)
	Update(context.Context, obj, metav1.UpdateOptions) (obj, error)
	Create(context.Context, obj, metav1.CreateOptions) (obj, error)
	Watch(ctx context.Context, opts metav1.ListOptions) (watch.Interface, error)
	Delete(context.Context, string, metav1.DeleteOptions) error
	DeleteCollection(ctx context.Context, opts metav1.DeleteOptions, listOpts metav1.ListOptions) error
}

type AmeGenClient[obj any] struct {
	Cli AmeKubeClient[obj]
}

func NewAmeGenClient[obj any](cli AmeKubeClient[obj]) AmeGenClient[obj] {
	return AmeGenClient[obj]{
		Cli: cli,
	}
}

func (c AmeGenClient[obj]) WaitForCondWithTimout(ctx context.Context, name string, verify func(obj) bool, timeout time.Duration) error {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	return c.WaitForCond(ctx, name, verify)
}

func (c AmeGenClient[obj]) WaitForCond(ctx context.Context, name string, verify func(obj) bool) error {
	objChan, err := c.WatchObj(ctx, name)
	if err != nil {
		return err
	}

	for {
		select {
		case <-ctx.Done():
			return fmt.Errorf("from WaitForCondition, failed to match condition within timeout")
		case o := <-objChan:
			if verify(o) {
				return nil
			}
		}
	}
}

func (c AmeGenClient[obj]) IsCondition(ctx context.Context, name string, verify func(obj) bool) (bool, error) {
	kubeObj, err := c.Cli.Get(ctx, name, metav1.GetOptions{})
	if err != nil {
		return false, err
	}

	return verify(kubeObj), nil
}

func (c AmeGenClient[obj]) WatchObj(ctx context.Context, name string) (<-chan obj, error) {
	opts, err := GenNameListOpts(name)
	if err != nil {
		return nil, err
	}

	return c.Watch(ctx, opts)
}

func (c AmeGenClient[obj]) Watch(ctx context.Context, opts metav1.ListOptions) (<-chan obj, error) {
	watcher, err := c.Cli.Watch(ctx, opts)
	if err != nil {
		return nil, err
	}

	objChan := make(chan obj)
	results := watcher.ResultChan()

	go func() {
		defer close(objChan)

		for {
			select {
			case <-ctx.Done():
				return
			case e, ok := <-results:
				if !ok {
					return
				}

				objInstans, ok := e.Object.(obj)
				// TODO: How will we handle if ok=false
				if ok {
					objChan <- objInstans
				}
			}
		}
	}()

	return objChan, nil
}

func (c AmeGenClient[obj]) Create(ctx context.Context, o obj) (obj, error) {
	return c.Cli.Create(ctx, o, metav1.CreateOptions{})
}

func (c AmeGenClient[obj]) Get(ctx context.Context, name string) (obj, error) {
	return c.Cli.Get(ctx, name, metav1.GetOptions{})
}

func (c AmeGenClient[obj]) DeleteCollection(ctx context.Context, delOpts metav1.DeleteOptions, listOpts metav1.ListOptions) error {
	return c.Cli.DeleteCollection(ctx, delOpts, listOpts)
}

func WaitForPodPhase(ctx context.Context, pods clientv1.PodInterface, p *v1.Pod, targetPhase v1.PodPhase, timeout time.Duration) error {
	ameGenCli := NewAmeGenClient[*v1.Pod](pods)
	return ameGenCli.WaitForCondWithTimout(ctx, p.GetName(), func(p *v1.Pod) bool {
		return p.Status.Phase == targetPhase
	}, timeout)
}

func GenTaskPhaseValidator(phase v1alpha1.TaskPhase) func(*v1alpha1.Task) bool {
	return func(t *v1alpha1.Task) bool {
		return t.Status.Phase == phase
	}
}

func IsTaskRunning(t *v1alpha1.Task) bool {
	return t.Status.Phase == v1alpha1.TaskRunning
}

func GenNameListOpts(name string) (metav1.ListOptions, error) {
	selector, err := genNameSelector(name)
	if err != nil {
		return metav1.ListOptions{}, err
	}

	return metav1.ListOptions{
		FieldSelector: selector.String(),
	}, nil
}

func genNameSelector(name string) (fields.Selector, error) {
	return fields.ParseSelector("metadata.name=" + name)
}

func GenTaskPodSelector(name string) (labels.Selector, error) {
	return labels.Parse(fmt.Sprintf("%s=%s", workflows.TaskLabelKey, name))
}
