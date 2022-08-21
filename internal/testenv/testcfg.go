package ametesting

import (
	"context"

	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/config"
	"teainspace.com/ame/internal/testcfg"
	"teainspace.com/ame/server/storage"
)

const (
	EchoProjectDir = "../../test_data/test_projects/echo"
)

func SetupCluster(ctx context.Context, cfg testcfg.TestEnvConfig) (storage.Storage, error) {
	store, err := storage.SetupStoreage(ctx, cfg.BucketName, cfg.ObjectStorageEndpoint)
	if err != nil {
		return nil, err
	}

	kubeCfg, err := clients.KubeClientFromConfig()
	if err != nil {
		return nil, err
	}

	err = ClearTasksInCluster(ctx, clients.TasksClientFromConfig(kubeCfg, cfg.Namespace))
	if err != nil {
		return nil, err
	}

	return store, nil
}

func ClearTasksInCluster(ctx context.Context, tasks v1alpha1.TaskInterface) error {
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

func LoadCliConfigToEnv(cfg testcfg.TestEnvConfig) error {
	return config.LoadCliCfgToEnv(config.CliConfig{
		AuthToken:   cfg.AuthToken,
		AmeEndpoint: cfg.AmeServerEndpoint,
	})
}
