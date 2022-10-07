package commands

import (
	"fmt"
	"log"
	"os"
	"strings"

	"github.com/AlecAivazis/survey/v2"
	"github.com/spf13/cobra"
	v1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	"sigs.k8s.io/yaml"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/ameproject"
)

func attachCreateTask(parentCmd *cobra.Command) *cobra.Command {
	parentCmd.AddCommand(&cobra.Command{
		Use:   "task",
		Short: "create a task",
		Long:  "create a task",
		Run:   createTask,
	})

	return parentCmd
}

func createTask(cmd *cobra.Command, args []string) {
	cfg, err := ameproject.ReadProjectFile(".")
	if err != nil && os.IsNotExist(err) {
		fmt.Println("No valid ame file was found, please run 'ame create project' to create one.")
		return
	}

	if err != nil {
		log.Fatal(err)
	}

	var res int
	err = survey.AskOne(&survey.Select{Message: "Please select what type of Task you want to create:", Options: []string{"Single step Task", "Multi step Task pipeline"}}, &res)
	if err != nil {
		log.Fatal(err)
	}

	var taskName string
	var task v1alpha1.TaskSpec
	if res == 0 {
		taskCfg, err := constructTask()
		if err != nil {
			log.Fatal(err)
		}

		task = v1alpha1.TaskSpec{
			RunCommand: taskCfg.RunCommand,
			Env:        taskCfg.Env,
			Secrets:    taskCfg.Secrets,
			Resources:  taskCfg.Resources,
		}

		taskName = taskCfg.TaskName
	} else if res == 1 {
		err = survey.AskOne(&survey.Input{Message: "What should the pipline be called:"}, &taskName)
		if err != nil {
			log.Fatal(err)
		}

		fmt.Println("Configure the first step:")

		steps, err := continuosInput(constructPipeline, "Add a step?", false)
		if err != nil {
			log.Fatal(err)
		}

		task.Pipeline = steps
	}

	fBuilder, err := ameproject.BuilderFromProjectFile(cfg)
	if err != nil {
		log.Fatal(err)
	}

	taskBytes, err := yaml.Marshal(&task)
	if err != nil {
		fmt.Println(err)
		return
	}

	taskString := strings.ReplaceAll(string(taskBytes), "source: {}", "")

	fmt.Println("This task will be saved in the project file (ame.yaml):")
	fmt.Println("Name: ", taskName)
	fmt.Println(taskString)

	fBuilder.AddTaskSpecs(ameproject.TaskSpecs{
		ameproject.TaskSpecName(taskName): &task,
	})

	err = ameproject.WriteToProjectFile("./", fBuilder.Build())
	if err != nil {
		log.Fatal(err)
	}
}

func continuosInput[K any](inputConstructor func() (K, error), msg string, askFirstTime bool) ([]K, error) {
	addOneMore := true
	var results []K

	if askFirstTime {
		err := survey.AskOne(&survey.Confirm{
			Message: msg,
			Default: false,
		}, &addOneMore)
		if err != nil {
			return nil, err
		}
	}

	for addOneMore {
		res, err := inputConstructor()
		if err != nil {
			return nil, err
		}

		results = append(results, res)

		err = survey.AskOne(&survey.Confirm{
			Message: msg,
			Default: false,
		}, &addOneMore)
		if err != nil {
			return nil, err
		}

	}

	return results, nil
}

func constructPipeline() (v1alpha1.PipelineStep, error) {
	newTask, err := constructTask()
	if err != nil {
		return v1alpha1.PipelineStep{}, err
	}

	return v1alpha1.PipelineStep{
		TaskName:   newTask.TaskName,
		RunCommand: newTask.RunCommand,
		Env:        newTask.Env,
		Secrets:    newTask.Secrets,
	}, nil
}

type taskCfg struct {
	v1alpha1.TaskSpec
	TaskName string
	taskResources
}

type taskResources struct {
	Cpu    string
	Memory string
	Gpu    string
}

func constructTask() (*taskCfg, error) {
	qs := genTaskQuestions()
	var cfg taskCfg
	err := survey.Ask(qs, &cfg)
	if err != nil {
		return nil, err
	}

	var taskResources taskResources
	err = survey.Ask(genResourceRequestions(), &taskResources)
	if err != nil {
		return nil, err
	}

	envVars, err := continuosInput(askForEnvVar, "Would you like to add an environment variable?", true)
	if err != nil {
		return nil, err
	}

	secrets, err := continuosInput(askForSecret, "Would you like to add a secret?", true)
	if err != nil {
		return nil, err
	}

	cfg.Env = envVars
	cfg.Secrets = secrets

	cfg.Resources = make(v1.ResourceList)

	cfg.Resources["cpu"] = resource.MustParse(taskResources.Cpu)
	cfg.Resources["memory"] = resource.MustParse(taskResources.Memory)
	if taskResources.Gpu != "0" {
		cfg.Resources["nvidia.com/gpu"] = resource.MustParse(taskResources.Gpu)
	}

	return &cfg, nil
}

func genResourceRequestions() []*survey.Question {
	resourceValidator := func(ans interface{}) error {
		str, ok := ans.(string)
		if !ok {
			return fmt.Errorf("expecte a string")
		}

		_, err := resource.ParseQuantity(str)
		return err
	}

	return []*survey.Question{
		{
			Name:     "Cpu",
			Prompt:   &survey.Input{Message: "Cpu: ", Default: "2"},
			Validate: resourceValidator,
		},
		{
			Name:     "Memory",
			Prompt:   &survey.Input{Message: "Memory: ", Default: "4Gi"},
			Validate: resourceValidator,
		},
		{
			Name:     "Gpu",
			Prompt:   &survey.Input{Message: "Gpu: ", Default: "0"},
			Validate: resourceValidator,
		},
	}
}

func genTaskQuestions() []*survey.Question {
	qs := []*survey.Question{
		{
			Name:     "TaskName",
			Prompt:   &survey.Input{Message: "Task name:"},
			Validate: survey.Required,
		},
		{
			Name:     "RunCommand",
			Prompt:   &survey.Input{Message: "Command:"},
			Validate: survey.Required,
		},
	}

	return qs
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

func askForSecret() (v1alpha1.TaskSecret, error) {
	qs := []*survey.Question{
		{
			Name: "Name",
			Prompt: &survey.Input{
				Message: "Secret name:",
			},
			Validate: survey.Required,
		},

		{
			Name: "EnvKey",
			Prompt: &survey.Input{
				Message: "Environment key:",
			},
			Validate: survey.Required,
		},
	}

	var envVar v1alpha1.TaskSecret
	err := survey.Ask(qs, &envVar)
	if err != nil {
		return v1alpha1.TaskSecret{}, err
	}

	return envVar, nil
}
