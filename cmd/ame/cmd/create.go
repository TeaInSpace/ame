package commands

import (
	"os"

	"github.com/spf13/cobra"
	"teainspace.com/ame/internal/ameproject"
)

func attachCreate(rootCmd *cobra.Command) *cobra.Command {
	createCmd := &cobra.Command{
		Use:   "create",
		Short: "Create objects in AME",
		Long:  "Create objects in AME",
		Run:   nil,
	}

	createCmd = attachCreateTask(createCmd)
	createCmd = attachCreateProject(createCmd)

	rootCmd.AddCommand(createCmd)
	return rootCmd
}

func getDirectoryName() string {
	wd, err := os.Getwd()
	if err == nil {
		return ameproject.ProjectNameFromDir(wd)
	}
	return ""
}
