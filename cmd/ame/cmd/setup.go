package commands

import (
	"log"

	"github.com/AlecAivazis/survey/v2"
	"teainspace.com/ame/internal/config"

	"github.com/spf13/cobra"
)

func attachSetup(rootCmd *cobra.Command) *cobra.Command {
	rootCmd.AddCommand(
		&cobra.Command{
			Use:   "setup",
			Short: "Connect the CLI with an AME server",
			Long:  "Connect the CLI with an AME server",
			Run:   setupCliConfig,
		},
	)

	return rootCmd
}

// setupCliConfig prompts the user for the required input to configure the CLI
// and stores the CLI's config in the user's home directory.
func setupCliConfig(cmd *cobra.Command, args []string) {
	qs := []*survey.Question{
		{
			Name:     "AuthToken",
			Prompt:   &survey.Password{Message: "auth token:"},
			Validate: survey.Required,
		},
		{
			Name:     "AmeEndpoint",
			Prompt:   &survey.Input{Message: "Endpoint:"},
			Validate: survey.Required,
		},
	}

	cfg := config.CliConfig{}

	err := survey.Ask(qs, &cfg)
	if err != nil {
		log.Fatal(err)
	}

	err = config.SaveCliCfg(cfg)
	if err != nil {
		log.Fatal(err)
	}
}
