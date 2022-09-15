package commands

import (
	"fmt"
	"log"

	"github.com/spf13/cobra"
	"teainspace.com/ame/internal/ameproject"
)

func attachExec(rootCmd *cobra.Command) *cobra.Command {
	rootCmd.AddCommand(&cobra.Command{
		Use:   "exec",
		Short: "Exec executes a task",
		Long:  "Exec executes a task",
		Run:   execTask,
	})

	return rootCmd
}

func execTask(cmd *cobra.Command, args []string) {
	if len(args) != 1 {
		log.Fatalf("expected the task name but got: %v arguments instead", len(args))
	}

	p, err := ameproject.ProjectFromWd(cmd.Context())
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println("setting token", p.AuthToken)

	taskName := args[0]

	_, err = p.UploadAndRunSpec(cmd.Context(), taskName)
	if err != nil {
		log.Fatal("failed to run task: ", err)
	}
}
