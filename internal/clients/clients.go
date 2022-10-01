package clients

import (
	"k8s.io/client-go/kubernetes"
	v1 "k8s.io/client-go/kubernetes/typed/core/v1"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/tools/clientcmd"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/common"

	argo "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
)

// TODO: should we handle errors when creating clients instead of dying?

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

func CronWorkflowsClientFromConfig(cfg *rest.Config, ns string) argo.CronWorkflowInterface {
	return argo.NewForConfigOrDie(cfg).CronWorkflows(ns)
}

func TasksClientFromConfig(cfg *rest.Config, ns string) v1alpha1.TaskInterface {
	return v1alpha1.NewForConfigOrDie(cfg).Tasks(ns)
}

func PodClientFromConfig(cfg *rest.Config, ns string) v1.PodInterface {
	return kubernetes.NewForConfigOrDie(cfg).CoreV1().Pods(ns)
}

func SecretsClientFromConfig(cfg *rest.Config, ns string) v1.SecretInterface {
	return kubernetes.NewForConfigOrDie(cfg).CoreV1().Secrets(ns)
}

func RecTasksClientFromConfig(cfg *rest.Config, ns string) v1alpha1.ReccurringTaskInterface {
	return v1alpha1.NewForConfigOrDie(cfg).ReccurringTasks(ns)
}

func GenericTaskClientFromConfig(cfg *rest.Config, ns string) common.AmeGenClient[*amev1alpha1.Task] {
	return common.NewAmeGenClient[*amev1alpha1.Task](TasksClientFromConfig(cfg, ns))
}
