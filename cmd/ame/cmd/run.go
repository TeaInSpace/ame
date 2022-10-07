package commands

import (
	"fmt"
	"log"
	"os"
	"strings"
	"time"

	"github.com/AlecAivazis/survey/v2"
	"github.com/briandowns/spinner"
	"github.com/spf13/cobra"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/ameproject"
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

func SelectTask(specs ameproject.TaskSpecs) (string, error) {
	var taskNames []string
	for k := range specs {
		taskNames = append(taskNames, string(k))
	}

	var name string
	err := survey.AskOne(&survey.Select{
		Message: "Please select a task:",
		Options: taskNames,
	}, &name)
	if err != nil {
		return "", err
	}
	return name, nil
}

// TODO: current have to pass run command in quotes "python train.py"
// TODO: handle errors gracefully in the CLI
// TODO: CLI seems to just finish early and pretent everything suceeded if a pod is pending, trying running a lot of
// pods after each other to reproduce this.
// TODO: Why does survey not block when being run during tests?

func runTask(cmd *cobra.Command, args []string) {
	p, err := ameproject.ProjectFromWd(cmd.Context())
	if err != nil && os.IsNotExist(err) {
		fmt.Println("No valid ame file was found, please run 'ame create project' to create one.")
		return
	}
	if err != nil {
		log.Fatalln(err)
	}

	if len(p.ProjectFile.Specs) == 0 {
		fmt.Println("There are no tasks for this project, please run 'ame create task' to create one.")
		return
	}

	var taskNames []string

	for k := range p.ProjectFile.Specs {
		taskNames = append(taskNames, string(k))
	}

	var name string
	if len(args) == 0 {
		name, err = SelectTask(p.ProjectFile.Specs)
		if err != nil {
			fmt.Println(err)
			return
		}
	} else {
		name = args[0]
		nameIsValid := false
		for _, tName := range taskNames {
			if name == tName {
				nameIsValid = true
			}
		}

		if !nameIsValid {
			fmt.Printf("The task %s does not exist in this project.\n", name)
			return
		}
	}

	s := spinner.New(spinner.CharSets[14], 100*time.Millisecond, spinner.WithWriter(os.Stderr))
	fmt.Println()
	s.Suffix = " Uploading project: " + p.Name
	s.Start()

	spec := p.ProjectFile.Specs[ameproject.TaskSpecName(name)]
	t := v1alpha1.NewTaskFromSpec(spec, name)

	projectTask, err := p.UploadAndRun(cmd.Context(), t)
	if err != nil {
		log.Fatalln(err)
	}

	s.Suffix = " Preparing execution environment"

	err = p.ProcessTaskLogs(cmd.Context(), projectTask, func(le *task.LogEntry) error {
		if !strings.Contains(le.Content, "s3") && !strings.Contains(le.Content, "argo") && !strings.Contains(le.Content, "WARNING: Exiting") && !strings.Contains(le.Content, "Uploading artifacts") {
			s.Suffix = " Executing"
			s.Stop()
			fmt.Println(le.Content)
			s.Start()
		}
		return nil
	})
	if err != nil {
		log.Fatalln(err)
	}

	projectTask, err = p.GetTask(cmd.Context(), projectTask.GetName(), projectTask.GetNamespace())
	if err != nil {
		fmt.Println("failed to retrieve Task object: ", projectTask.GetName(), err)
		return
	}

	if projectTask.Status.Phase != v1alpha1.TaskSucceeded {
		s.Stop()
		fmt.Printf("The Task failed with the message:\n %s\n", projectTask.Status.Reason)
		return
	}

	fmt.Println("The task finished successfully!")

	artifacts, err := p.GetArtifacts(cmd.Context(), projectTask.GetName())
	if err != nil {
		log.Default().Fatal(err)
	}

	if len(artifacts) > 0 {

		s.Suffix = " Saving artifacts"
		err = dirtools.PopulateDir(".", artifacts)
		if err != nil {
			log.Default().Fatalf("failed to save artifacts due to error: %v", err)
		}
		s.FinalMSG = "✓ Artifacts saved"
	} else {
		s.FinalMSG = "✓ Done"
	}

	s.Stop()
}
