package commands

import "github.com/spf13/cobra"

func newRootCmdWithSubCmds() *cobra.Command {
	rootCmd := cobra.Command{
		Use:   "ame",
		Short: "ame is an awesome! MLOPS platform",
		Long:  "ame is an awesome! MLOPS platform",
	}
	return attachSubCommands(&rootCmd)
}

func attachSubCommands(rootCmd *cobra.Command) *cobra.Command {
	rootCmd = attachCreate(rootCmd)
	rootCmd = attachSetup(rootCmd)
	rootCmd = attachSchedule(rootCmd)
	return attachRun(rootCmd)
}

func Execute() {
	newRootCmdWithSubCmds().Execute()
}
