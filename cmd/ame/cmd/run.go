package commands

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/AlecAivazis/survey/v2"
	"github.com/briandowns/spinner"
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

func shouldAProjectBeCreated() (bool, error) {
	ok, err := ameproject.ValidProjectCfgExists(".")
	if err != nil {
		return false, err
	}

	if !ok {
		var resp bool
		err = survey.AskOne(&survey.Confirm{
			Message: "Would you like to setup a project?",
			Default: true,
		}, &resp)

		if err != nil {
			return false, err
		}

		return resp, nil
	}

	return false, nil
}

// TODO: current have to pass run command in quotes "python train.py"
// TODO: handle errors gracefully in the CLI
// TODO: CLI seems to just finish early and pretent everything suceeded if a pod is pending, trying running a lot of
// pods after each other to reproduce this.
// TODO: Why does survey not block when being run during tests?

func runTask(cmd *cobra.Command, args []string) {
	ctx := context.Background()
	cfg, err := config.GenCliConfig()
	// TODO: handle missing configuration gracefully
	if err != nil {
		log.Fatalln("It looks like no CLI configuration is present, got error: ", err)
	}

	ok, err := shouldAProjectBeCreated()
	if err != nil {
		// TODO: determine how to check for survey EOF so we don't break TestCanDownloadArtifacts when failing on this error.
		log.Println(err)
	}

	if ok {
		createProjectFile(cmd, args)
	}

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
	s := spinner.New(spinner.CharSets[14], 100*time.Millisecond, spinner.WithWriter(os.Stderr))
	fmt.Println()
	s.Suffix = " Uploading project: " + p.Name
	s.Start()

	// TODO: we need to sort out how the user is supposed to execute a task from the project file
	// vs adhoc tasks.
	t := v1alpha1.NewTask(args[0], p.Name)
	if ok {
		projectCfg, err := ameproject.ReadProjectFile(".")
		if err != nil {
			log.Fatal(err)
		}

		for k := range projectCfg.Specs {
			t.Spec.Env = projectCfg.Specs[k].Env
		}

	}

	projectTask, err := p.UploadAndRun(ctx, t)
	if err != nil {
		log.Fatal(err)
	}

	s.Suffix = " Preparing execution environment"

	err = p.ProcessTaskLogs(ctx, projectTask, func(le *task.LogEntry) error {
		if s.Active() {
			s.Stop()
			fmt.Println("Your task will be executed!", args[0])
		}

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
