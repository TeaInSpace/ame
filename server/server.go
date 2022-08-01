package main

import (
	"context"
	"os"

	"k8s.io/client-go/rest"
	task "teainspace.com/ame/server/cmd"
)

const (
	AME_SERVER_PORT_ENV_VAR_KEY = "AME_SERVER_PORT"
	AME_SEVER_DEFAULT_PORT      = "3342"
)

func serverPort() string {
	ameServerPort := os.Getenv(AME_SERVER_PORT_ENV_VAR_KEY)
	if ameServerPort == "" {
		ameServerPort = AME_SEVER_DEFAULT_PORT
	}

	return ameServerPort
}

func main() {
	ctx := context.Background()
	inclusterConfig, err := rest.InClusterConfig()
	if err != nil {
		panic(err)
	}
	_, serve, err := task.Run(ctx, inclusterConfig, serverPort())
	if err != nil {
		panic(err)
	}

	err = serve()
	if err != nil {
		panic(err)
	}
}
