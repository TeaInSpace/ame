package config

import (
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCanLoadCliConfigFromEnv(t *testing.T) {
	// PrepTmpCfgDir ensures that the config will not be read from the user's config directory
	// but a fake temporary config directory instread. This ensures there is not CLI configuration for
	// the config package to read. The CliConfig field are left empty to ensure no config file is created,
	// as this test is only focused on reading from environment variables without influence of any sort from
	// a config file.
	err := PrepTmpCfgDir(CliConfig{})
	if err != nil {
		t.Error(err)
	}

	expectedCfg := CliConfig{
		AuthToken:   "atoken",
		AmeEndpoint: "https://some.endpoint.com",
	}

	err = LoadCliCfgToEnv(expectedCfg)
	if err != nil {
		t.Error(err)
	}
	defer ClearCliCfgFromEnv()

	cfg, err := GenCliConfig()
	if err != nil {
		t.Error(err)
	}

	diff := cmp.Diff(expectedCfg, cfg)
	if diff != "" {
		t.Errorf("Got getConfig()=%+v, expected %+v, diff:  %s", cfg, expectedCfg, diff)
	}
}

func TestCanLoadCliConfigFromApplicationConfigDirectory(t *testing.T) {
	correctCliConfig := CliConfig{
		AuthToken:   "token",
		AmeEndpoint: "https://myend.com",
	}

	// The default application config directory location is overwritten here,
	// to keep the test isolated and avoid overwriting configuration in the user's actual
	// config directory.
	err := PrepTmpCfgDir(correctCliConfig)
	if err != nil {
		t.Error(err)
	}

	generatedCfg, err := GenCliConfig()
	if err != nil {
		t.Error(err)
	}

	diff := cmp.Diff(correctCliConfig, generatedCfg)
	if diff != "" {
		t.Errorf("Got GenCliConfig()=%+v, expected %+v, diff: %s", generatedCfg, correctCliConfig, diff)
	}
}

func TestEnvVarPrecedenceOverCfgFile(t *testing.T) {
	correctCliCfg := CliConfig{
		AuthToken:   "correcttoken",
		AmeEndpoint: "https://correct.endpoint.com",
	}

	err := PrepTmpCfgDir(CliConfig{
		AuthToken:   "myfavtoken",
		AmeEndpoint: "https://myfacendpoint.com",
	})
	if err != nil {
		t.Error(err)
	}

	err = LoadCliCfgToEnv(correctCliCfg)
	if err != nil {
		t.Error(err)
	}
	defer ClearCliCfgFromEnv()

	cliCfg, err := GenCliConfig()
	if err != nil {
		t.Error(err)
	}

	diff := cmp.Diff(correctCliCfg, cliCfg)
	if diff != "" {
		t.Errorf("GenCliConfig()=%+v, but expected %+v, diff: %s", cliCfg, correctCliCfg, diff)
	}
}

func TestGenCliConfigFailsForMissingRequiredField(t *testing.T) {
	// Inorder to test behavior when no config is present, it is important
	// that no config file is avaible.
	err := PrepTmpCfgDir(CliConfig{})
	if err != nil {
		t.Error(err)
	}

	cases := []struct {
		name string
		cfg  CliConfig
	}{
		{
			name: "Missing AuthToken",
			cfg:  CliConfig{AmeEndpoint: "someendpoint"},
		},
		{
			name: "Missing AmeEndpoint",
			cfg:  CliConfig{AuthToken: "sometoken"},
		},
		{
			name: "Missing everything",
			cfg:  CliConfig{},
		},
	}

	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			// En sure that test environments are isolated.
			err := ClearCliCfgFromEnv()
			if err != nil {
				t.Error(err)
			}

			err = LoadCliCfgToEnv(c.cfg)
			if err != nil {
				t.Error()
			}

			_, err = GenCliConfig()
			if err == nil {
				t.Errorf("Got %v error from GenCliCfg, but expected a non nil error", err)
			}
		})
	}
}
