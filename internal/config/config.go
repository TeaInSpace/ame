package config

import (
	"errors"
	"fmt"
	"os"
	"path"

	"github.com/spf13/viper"
)

const (
	// AmeCliEnvKeyPrefix contains the prefix for all AME CLI related environment variables.
	AmeCliEnvKeyPrefix      = "AME_CLI"
	AmeCliEnvKeyAuthToken   = "AUTHTOKEN"
	AmeCliEnvKeyAmeEndpoint = "AMEENDPOINT"
	AmeCfgDirName           = "ame"

	// AmeConfigFileHame defines the expected name of the config which the AME CLI will read from the
	// user's default config file location.
	AmeConfigFileHame = "cli.yaml"
)

// CliConfig contains the configuration required for the AME CLI.
// AuthToken is the bearer token used by the CLI for the AME
// gRPC API authentication. Note that the token should no included bearer in front,
// but just be the token itself.
// AmeEndpoint is the endpoint targeted by the CLI when making gRPC requests. The endpoint
// should be complete for example: https://myendpoint.com:3829
type CliConfig struct {
	AuthToken   string
	AmeEndpoint string
}

func ameCfgFilePath() (string, error) {
	cfgDir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}

	return path.Join(path.Join(cfgDir, AmeCfgDirName), AmeConfigFileHame), nil
}

// GenCliConfig generates a CliConfig using based on environment variables,
// config files.
// The expected prefix  for environment variables is defined by the const
// AmeCliEnvKeyPrefix.
// A CliConfig struct is returned with the values from the environment.
func GenCliConfig() (CliConfig, error) {
	v := viper.New()
	v.SetEnvPrefix(AmeCliEnvKeyPrefix)

	err := v.BindEnv(AmeCliEnvKeyAuthToken)
	if err != nil {
		return CliConfig{}, err
	}

	err = v.BindEnv(AmeCliEnvKeyAmeEndpoint)
	if err != nil {
		return CliConfig{}, err
	}

	cfgFilePath, err := ameCfgFilePath()
	if err != nil {
		return CliConfig{}, err
	}

	v.SetConfigFile(cfgFilePath)
	err = v.ReadInConfig()

	// It is important that an error is not thrown simply due to a config file not being present
	// as it perfectly valid to configure the CLI using environment variables or flags.
	if err != nil && !errors.As(err, &viper.ConfigFileNotFoundError{}) && !os.IsNotExist(err) {
		return CliConfig{}, err
	}

	// err may not be nil after the previous check and must therefore be reset,
	// to avoid interfering with error checks below.
	err = nil

	var c CliConfig
	err = v.UnmarshalExact(&c)
	if err != nil {
		return CliConfig{}, err
	}

	// An empty auth token is not considered a valid configuration for the CLI as
	// no API requests can be made without a token.
	if c.AuthToken == "" {
		return CliConfig{}, fmt.Errorf("authentication token was not set")
	}

	// An empty endpoint is not considered a valid configration for the CLI as
	// no API requests can be made without having an endpoint to make requests
	// against.
	if c.AmeEndpoint == "" {
		return CliConfig{}, fmt.Errorf("AME endpoint was not set")
	}

	return c, nil
}

// SaveCliCfg saves cfg to the default AME CLI config file location.
// If the AME application config directory does not exist, it will
// be created.
// A non nil error is returned if the operation fails.
func SaveCliCfg(cfg CliConfig) error {
	err := ensureCfgDirExists()
	if err != nil {
		return err
	}

	cfgPath, err := ameCfgFilePath()
	if err != nil {
		return err
	}

	return writeCliConfigToFile(cfgPath, cfg)
}

func getAmeCfgDir() (string, error) {
	userCfgDir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}

	return path.Join(userCfgDir, AmeCfgDirName), nil
}

// ensureCfgDirExists create's the AME specific application configure directory
// if it does not already exist.
// A non nil error is returned if the operation fails.
func ensureCfgDirExists() error {
	cfgDir, err := getAmeCfgDir()
	if err != nil {
		return err
	}

	_, err = os.Stat(cfgDir)
	if os.IsNotExist(err) {
		// Note that it is important to us Mkdir and not MkdirAll here
		// as we do not want to make parent directories as the config directory
		// should always be present on a user's system if it is not something is
		// probably wrong and an error should be returned.
		return os.Mkdir(cfgDir, 0o777)
	}

	return err
}

// writeCliConfigToFile writes cfg to the location specified with filePath
// and returns and non nil error if the operation fails.
// If there is an existing file it will be overwritten and if no file
// exists a new file will be created.
func writeCliConfigToFile(filePath string, cfg CliConfig) error {
	v := viper.New()
	v.Set(AmeCliEnvKeyAuthToken, cfg.AuthToken)
	v.Set(AmeCliEnvKeyAmeEndpoint, cfg.AmeEndpoint)
	return v.WriteConfigAs(filePath)
}

func cliEnvVarKey(key string) string {
	return fmt.Sprintf("%s_%s", AmeCliEnvKeyPrefix, key)
}

// LoadCliCfgToEnv sets the corresponding environment variables for each
// field in cfg.
// A non nil error is returned if the operation fails.
func LoadCliCfgToEnv(cfg CliConfig) error {
	err := os.Setenv(cliEnvVarKey(AmeCliEnvKeyAuthToken), cfg.AuthToken)
	if err != nil {
		return err
	}

	err = os.Setenv(cliEnvVarKey(AmeCliEnvKeyAmeEndpoint), cfg.AmeEndpoint)
	if err != nil {
		return err
	}

	return nil
}

// ClearCliCfgFromEnv unsets all environment variables used by the
// AME CLI.
// A non nil error is returned if the operation fails.
func ClearCliCfgFromEnv() error {
	err := os.Unsetenv(cliEnvVarKey(AmeCliEnvKeyAuthToken))
	if err != nil {
		return err
	}

	err = os.Unsetenv(cliEnvVarKey(AmeCliEnvKeyAmeEndpoint))
	if err != nil {
		return err
	}

	return nil
}

// PrepTmpCfgDir prepares a temporary directory to be used as the default config directory
// , $XDG_CONFIG_HOME is overwritten to point to the temporary directory and a subdirectory
// is created to act as the AME specific configuration directory, to ensure the same structure
// as the CLI would normally be dealing with in a real usecase. see os.MkDirTemp for more details
// on how temporary directories are created. If all fields in cfg are empty, the cli.yaml config
// is not created and the applicatioh specific config directory is not created.
// A non nil error is returned if the operation fails.
// Example:
// Normal structure: /home/myuser/.config/ame/cli.yaml
// Temporay dir structure: $DEFAULT_TMP_DIR/tmpcfgdir2343243/ame/cli.yaml
func PrepTmpCfgDir(cfg CliConfig) error {
	tmpDir, err := os.MkdirTemp("", "tmpcfgdir")
	if err != nil {
		return err
	}

	// It is important to overwrite XDG_CONFIG_HOME before
	// calling ensureCfgDirExists or ameCfgFilePath as they rely
	// on os.UserConfigDir which will read from XDG_HOME_DIR.
	err = os.Setenv("XDG_CONFIG_HOME", tmpDir)
	if err != nil {
		return err
	}

	// If cfg is empty there is no reason to save it.
	// This allows for prepareing a temporary config directory
	// while not saving a cli.yaml file to it.
	if cfg.AmeEndpoint == "" && cfg.AuthToken == "" {
		return nil
	}

	err = SaveCliCfg(cfg)
	if err != nil {
		return err
	}

	return nil
}
