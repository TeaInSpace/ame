package commands

import (
	"log"
	"os"

	"github.com/AlecAivazis/survey/v2"
	"github.com/spf13/cobra"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/ameproject"
)

func attachCreate(rootCmd *cobra.Command) *cobra.Command {
	rootCmd.AddCommand(&cobra.Command{
		Use:   "create",
		Short: "short desp",
		Long:  "Long desp",
		Run:   createProjectFile,
	})

	return rootCmd
}

func getDirectoryName() string {
	wd, err := os.Getwd()
	if err == nil {
		return ameproject.ProjectNameFromDir(wd)
	}
	return ""
}

func createProjectFile(cmd *cobra.Command, args []string) {
	qs, err := genQuestions(args)
	if err != nil {
		log.Fatalln(err)
	}

	cfg := &struct {
		ProjectName string
		TaskName    string
		Command     string
	}{}
	err = survey.Ask(qs, cfg)
	if err != nil {
		log.Fatal(err)
	}

	var envVars []v1alpha1.TaskEnvVar
	var addEnvironmentVar bool
	for {
		err = survey.AskOne(&survey.Confirm{
			Message: "Would you like to add an environment variable?",
			Default: false,
		}, &addEnvironmentVar)

		if err != nil {
			log.Fatal(err)
		}

		if !addEnvironmentVar {
			break
		}

		envVar, err := askForEnvVar()
		if err != nil {
			log.Fatalln(err)
		}

		envVars = append(envVars, envVar)
	}

	fBuilder := ameproject.NewProjectFileBuilder()
	fBuilder.SetProjectName(cfg.ProjectName)
	fBuilder.AddTaskSpecs(ameproject.TaskSpecs{
		ameproject.TaskSpecName(cfg.TaskName): &v1alpha1.TaskSpec{
			RunCommand: cfg.Command,
			ProjectId:  cfg.ProjectName,
			Env:        envVars,
		},
	})

	err = ameproject.WriteToProjectFile("./", fBuilder.Build())
	if err != nil {
		log.Fatal(err)
	}
}

func genQuestions(args []string) ([]*survey.Question, error) {
	qs := []*survey.Question{
		{
			Name:     "TaskName",
			Prompt:   &survey.Input{Message: "Task name:"},
			Validate: survey.Required,
		},
		{
			Name:     "Command",
			Prompt:   &survey.Input{Message: "Command:"},
			Validate: survey.Required,
		},
	}

	ok, err := ameproject.ValidProjectCfgExists(".")
	if err != nil {
		return nil, err
	}

	if !ok {
		qs = append([]*survey.Question{
			{
				Name: "ProjectName",
				Prompt: &survey.Input{
					Message: "Project name:", Default: getDirectoryName(),
					Help: "The project name is used to uniquely identify the context required by tasks for this project.",
				},
				Validate: survey.Required,
			},
		}, qs...)
	}

	// TODO: get rid of this hack when rearranging the CLI.
	if len(args) == 1 {
		qs[2].Prompt = &survey.Input{Message: "Command:", Default: args[0]}
	}
	return qs, nil
}

func askForEnvVar() (v1alpha1.TaskEnvVar, error) {
	qs := []*survey.Question{
		{
			Name: "Name",
			Prompt: &survey.Input{
				Message: "Variable name:",
			},
			Validate: survey.Required,
		},

		{
			Name: "Value",
			Prompt: &survey.Input{
				Message: "Variable value:",
			},
			Validate: survey.Required,
		},
	}

	var envVar v1alpha1.TaskEnvVar
	err := survey.Ask(qs, &envVar)
	if err != nil {
		return v1alpha1.TaskEnvVar{}, err
	}

	return envVar, nil
}
