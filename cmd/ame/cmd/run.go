package commands

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/ameproject"
	"teainspace.com/ame/internal/auth"
	"teainspace.com/ame/internal/config"
	"teainspace.com/ame/internal/dirtools"
	task "teainspace.com/ame/server/grpc"
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

// TODO: current have to pass run command in quotes "python train.py"
// TODO: handle errors gracefully in the CLI

func runTask(cmd *cobra.Command, args []string) {
	ctx := context.Background()
	cfg, err := config.GenCliConfig()
	// TODO: handle missing configuration gracefully
	if err != nil {
		log.Fatalln("It looks like no CLI configuration is present, got error: ", err)
	}

	fmt.Println("Your task will be executed!", args[0])

	// TODO: move grpc setup to a library package.
	var opts []grpc.DialOption
	opts = append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))

	conn, err := grpc.Dial(cfg.AmeEndpoint, opts...)
	if err != nil {
		log.Fatal(err)
	}

	wd, err := os.Getwd()
	if err != nil {
		log.Fatal(wd)
	}

	taskClient := task.NewTaskServiceClient(conn)
	p := ameproject.NewProjectForDir(wd, taskClient)

	// TODO: handle authtorization the context elegantly.
	ctx = auth.AuthorarizeCtx(ctx, cfg.AuthToken)

	projectTask, err := p.UploadAndRun(ctx, v1alpha1.NewTask(args[0], p.Name))
	if err != nil {
		log.Fatal(err)
	}

	err = p.ProcessTaskLogs(ctx, projectTask, func(le *task.LogEntry) error {
		fmt.Println(le.Content)
		return nil
	})

	if err != nil {
		log.Fatal(err)
	}

	log.Println("Fetching artifacts produced during task execution.")

	artifacts, err := ameproject.GetArtifacts(ctx, taskClient, projectTask.GetName())
	if err != nil {
		log.Default().Fatal(err)
	}

	artifactPaths := ""
	for _, ar := range artifacts {
		artifactPaths = fmt.Sprintf("%s, %s", artifactPaths, ar.Path)
	}
	log.Println("Writing artifacts to disk.", artifactPaths)

	err = dirtools.PopulateDir(wd, artifacts)
	if err != nil {
		log.Default().Fatalf("failed to save artifacts due to error: %v", err)
	}

	log.Println("Artifacts successfully saved")
}
