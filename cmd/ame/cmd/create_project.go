package commands

import (
	"fmt"
	"log"

	"github.com/AlecAivazis/survey/v2"
	"github.com/spf13/cobra"
	"teainspace.com/ame/internal/ameproject"
)

func attachCreateProject(parentCmd *cobra.Command) *cobra.Command {
	parentCmd.AddCommand(&cobra.Command{
		Use:   "project",
		Short: "create a project",
		Long:  "create a projet",
		Run:   createProjectFile,
	})

	return parentCmd
}

func createProjectFile(cmd *cobra.Command, args []string) {
	qs, err := genQuestions(args)
	if err != nil {
		log.Fatalln(err)
	}

	cfg := &struct {
		ProjectName string
	}{}

	err = survey.Ask(qs, cfg)
	if err != nil {
		log.Fatal(err)
	}

	fBuilder := ameproject.NewProjectFileBuilder()
	fBuilder.SetProjectName(cfg.ProjectName)

	err = ameproject.WriteToProjectFile("./", fBuilder.Build())
	if err != nil {
		log.Fatal(err)
	}
}

func genQuestions(args []string) ([]*survey.Question, error) {
	ok, err := ameproject.ValidProjectCfgExists(".")
	if err != nil {
		return nil, err
	}

	if ok {
		return nil, fmt.Errorf("a project file already exists")
	}

	qs := []*survey.Question{
		{
			Name: "ProjectName",
			Prompt: &survey.Input{
				Message: "Project name:", Default: getDirectoryName(),
				Help: "The project name is used to uniquely identify the context required by tasks for this project.",
			},
			Validate: survey.Required,
		},
	}

	return qs, nil
}
