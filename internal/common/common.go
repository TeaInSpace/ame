package common

import (
	"context"
	"time"

	v1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	clientv1 "k8s.io/client-go/kubernetes/typed/core/v1"
)

func WaitForPodPhase(ctx context.Context, pods clientv1.PodInterface, p *v1.Pod, targetPhase v1.PodPhase) error {
	for {
		taskPod, err := pods.Get(ctx, p.GetName(), metav1.GetOptions{})
		if err != nil {
			return err
		}

		if taskPod.Status.Phase == v1.PodRunning {
			return nil
		}

		time.Sleep(time.Millisecond * 50)
	}
}
