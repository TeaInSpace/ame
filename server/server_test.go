package main

import (
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestPortConfiguration(t *testing.T) {
	portEnvVarTests := []struct {
		in  string
		out string
	}{
		{"3424", "3424"},             // Set the env variable to a non-default port
		{"", AME_SEVER_DEFAULT_PORT}, // Leave the env variable empty and use the default port
		{"dfkeijd", "dfkeijd"},       // Provide malformed input and expected that input to be used
	}
	// Using the malformed input is important as the user should see the server fail when providing a
	// bad configuration. If the server and fell back to the default config it would look like everything
	// was fine.

	for _, tt := range portEnvVarTests {
		t.Run(tt.in, func(t *testing.T) {
			os.Setenv(AME_SERVER_PORT_ENV_VAR_KEY, tt.in)
			assert.Equal(t, tt.out, serverPort())
		})
	}

	os.Setenv(AME_SERVER_PORT_ENV_VAR_KEY, "")
}
