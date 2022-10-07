package commands

import (
	"fmt"
	"log"
	"os"

	"github.com/AlecAivazis/survey/v2"
	"github.com/spf13/cobra"
	"teainspace.com/ame/internal/ameproject"
)

func attachSchedule(rootCmd *cobra.Command) *cobra.Command {
	scheduleCmd := &cobra.Command{
		Use:   "schedule",
		Short: "Schedule task for recurring execution",
		Long:  "Schedule task for recurring execution",
		Run:   nil,
	}

	scheduleCmd = attachScheduleTask(scheduleCmd)

	rootCmd.AddCommand(scheduleCmd)
	return rootCmd
}

func attachScheduleTask(rootCmd *cobra.Command) *cobra.Command {
	scheduleCmd := &cobra.Command{
		Use:   "task",
		Short: "Schedule task for recurring execution",
		Long:  "Schedule task for recurring execution",
		Run:   scheduleTask,
	}

	scheduleCmd.Flags().StringP("repo", "r", "", "Provide the Git repository AME will clone and run the Task from")
	scheduleCmd.Flags().StringP("ref", "e", "", "Provide the Git reference AME will checkout after cloning the Git repository")
	scheduleCmd.Flags().StringP("task", "t", "", "Provide the Git reference AME will checkout after cloning the Git repository")
	scheduleCmd.Flags().StringP("schedule", "s", "", "Provide the cron schedule for the recurring Task")

	rootCmd.AddCommand(scheduleCmd)
	return rootCmd
}

func scheduleTask(cmd *cobra.Command, args []string) {
	p, err := ameproject.ProjectFromWd(cmd.Context())
	if err != nil && os.IsNotExist(err) {
		fmt.Println("No valid ame file was found, please run 'ame create project' to create one.")
		return
	}

	if err != nil {
		log.Fatalln(err)
	}

	schedule := cmd.Flag("schedule")
	gitRepo := cmd.Flag("repo")
	gitReference := cmd.Flag("ref")
	taskName := cmd.Flag("task")

	flags := []string{"schedule", "repo", "ref", "task"}

	scheduleCfg := struct {
		Schedule string `survey:"schedule"`
		Repo     string `survey:"repo"`
		Ref      string `survey:"ref"`
		Task     string `survey:"task"`
	}{
		schedule.Value.String(),
		gitRepo.Value.String(),
		gitReference.Value.String(),
		taskName.Value.String(),
	}

	var qs []*survey.Question
	for _, f := range flags {
		if cmd.Flag(f).Value.String() == "" {
			if f == "task" {
				continue
			}
			qs = append(qs, &survey.Question{
				Name: f,
				Prompt: &survey.Input{
					Message: fmt.Sprintf("Please provide the %s", f),
				},
			})
		}
	}

	err = survey.Ask(qs, &scheduleCfg)
	if err != nil {
		fmt.Println(err)
		return
	}

	name, err := SelectTask(p.ProjectFile.Specs)
	if err != nil {
		fmt.Println(err)
		return
	}
	spec, err := p.GetTaskSpec(name)
	if err != nil {
		fmt.Println(err)
		return
	}

	scheduleCfg.Task = name

	spec.Source.GitRepository = scheduleCfg.Repo
	spec.Source.GitReference = scheduleCfg.Ref

	_, err = p.ScheduleTask(cmd.Context(), *spec, scheduleCfg.Schedule)
	fmt.Println()
	if err != nil {
		fmt.Println("failed to schedule task, got error: ", err)
	} else {
		fmt.Printf("Scheduled task: %s with schedule: %s\n", scheduleCfg.Task, scheduleCfg.Schedule)
	}
}
