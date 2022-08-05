package commands

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/cmd/ame/filescanner"
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

const chunkSize = 64 * 1024

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

	t, err := filescanner.TarDirectory("./", []string{})
	if err != nil {
		log.Fatalln("Could not tar directory", err)
	}

	grpcCtx := metadata.AppendToOutgoingContext(ctx, task.MdKeyProjectName, currentDir)
	uploadClient, err := taskClient.FileUpload(grpcCtx)
	if err != nil {
		log.Fatalln(err)
	}

	for {
		nextChunk := make([]byte, chunkSize)
		_, err := t.Read(nextChunk)
		if err == io.EOF {
			status, err := uploadClient.CloseAndRecv()
			if err != nil {
				log.Println(err)
			}

			fmt.Println(status)
			break
		}

		if err != nil {
			log.Fatalln(err)
		}

		uploadClient.Send(&task.Chunk{
			Contents: nextChunk,
		})
	}

	_, err = taskClient.CreateTask(ctx, &task.TaskCreateRequest{Namespace: "ame-system", Task: &v1alpha1.Task{ObjectMeta: metav1.ObjectMeta{Name: currentDir}, Spec: v1alpha1.TaskSpec{RunCommand: args[0]}}})
	if err != nil {
		panic(err)
	}
}
