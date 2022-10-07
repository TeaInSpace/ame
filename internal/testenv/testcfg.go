package ametesting

import (
	"context"

	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/config"
	"teainspace.com/ame/internal/testcfg"
	"teainspace.com/ame/server/storage"
)

const (
	EchoProjectDir     = "../../test_data/test_projects/echo"
	EnvProjectDir      = "../../test_data/test_projects/env"
	ArtifactProjectDir = "../../test_data/test_projects/artifacts"
	PipelineProjectDir = "../../test_data/test_projects/pipeline"
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

	taskClient := clients.GenericTaskClientFromConfig(kubeCfg, cfg.Namespace)
	recTaskClient := clients.GenericRecurringTaskCLient(kubeCfg, cfg.Namespace)

	err = taskClient.DeleteCollection(ctx, v1.DeleteOptions{}, v1.ListOptions{})
	if err != nil {
		return nil, err
	}

	err = recTaskClient.DeleteCollection(ctx, v1.DeleteOptions{}, v1.ListOptions{})
	if err != nil {
		return nil, err
	}

	return store, nil
}

func LoadCliConfigToEnv(cfg testcfg.TestEnvConfig) error {
	return config.LoadCliCfgToEnv(config.CliConfig{
		AuthToken:   cfg.AuthToken,
		AmeEndpoint: cfg.AmeServerEndpoint,
	})
}
