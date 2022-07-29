package commands

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"teainspace.com/ame/api/v1alpha1"
	task "teainspace.com/ame/server/cmd"
)

func attachRun(rootCmd *cobra.Command) *cobra.Command {
	rootCmd.AddCommand(&cobra.Command{
		Use:   "run",
		Short: "short desp",
		Long:  "Long desp",
		Run:   runTask,
	})

	return rootCmd
}

func runTask(cmd *cobra.Command, args []string) {
	ctx := context.Background()
	fmt.Println("Your task will be executed!", args[0])

	var opts []grpc.DialOption
	opts = append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))

	conn, err := grpc.Dial("172.18.255.200:3342", opts...)
	if err != nil {
		panic(err)
	}
	taskClient := task.NewTaskServiceClient(conn)

	wd, err := os.Getwd()
	if err != nil {
		log.Fatalln(err)
	}

	currentDir := filepath.Base(wd)
	_, err = taskClient.CreateTask(ctx, &task.TaskCreateRequest{Namespace: "ame-system", Task: &v1alpha1.Task{ObjectMeta: metav1.ObjectMeta{Name: currentDir}, Spec: v1alpha1.TaskSpec{RunCommand: args[0]}}})
	if err != nil {
		panic(err)
	}
}
