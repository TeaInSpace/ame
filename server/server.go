package main

import (
	"k8s.io/client-go/rest"
	"teainspace.com/ame/server/cmd"
)

func main() {
	inclusterConfig, err := rest.InClusterConfig()
	if err != nil {
		panic(err)
	}

	_, serve, err := task.Run(inclusterConfig, 3000)
	if err != nil {
		panic(err)
	}

	err = serve()
	if err != nil {
		panic(err)
	}
}
