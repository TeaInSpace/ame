package clients

import (
	"k8s.io/client-go/kubernetes"
	v1 "k8s.io/client-go/kubernetes/typed/core/v1"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/tools/clientcmd"
	"teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"

	argo "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
)

func KubeClientFromConfig() (*rest.Config, error) {
	configLoadingRules := clientcmd.NewDefaultClientConfigLoadingRules()
	kubeConfig := clientcmd.NewNonInteractiveDeferredLoadingClientConfig(configLoadingRules, &clientcmd.ConfigOverrides{})
	config, err := kubeConfig.ClientConfig()
	if err != nil {
		return nil, err
	}

	return config, nil
}

func WorkflowsClientFromConfig(cfg *rest.Config, ns string) argo.WorkflowInterface {
	return argo.NewForConfigOrDie(cfg).Workflows(ns)
}

func TasksClientFromConfig(cfg *rest.Config, ns string) v1alpha1.TaskInterface {
	return v1alpha1.NewForConfigOrDie(cfg).Tasks(ns)
}

func PodClientFromConfig(cfg *rest.Config, ns string) v1.PodInterface {
	return kubernetes.NewForConfigOrDie(cfg).CoreV1().Pods(ns)
}
