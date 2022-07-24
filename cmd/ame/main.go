package main

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

func main() {
	rootCmd := &cobra.Command{
		Use:   "run",
		Short: "This command runs your project",
		Long:  "This is a longer description",
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Println("Your project entry: ", args)
		},
	}

	err := rootCmd.Execute()
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
