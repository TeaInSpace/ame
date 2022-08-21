package testcfg

import (
	"log"
	"os"
	"path"

	"github.com/Netflix/go-env"
	"github.com/joho/godotenv"
)

// testenvName contains the name of the .env file generate in the root
// of the project containing the necessary information to configure all
// of AME's componenets to work the test cluster. See the project's README
// for details on how to generate this file.
const testenvName = "test.env"

// A TestEnvConfig represents the configuration required to configure AME's components
// to run in the local test environment, generated with make deploy_local_cluster.
type TestEnvConfig struct {
	Namespace             string `env:"AME_NAMESPACE"`
	AmeServerEndpoint     string `env:"AME_SERVER_ENDPOINT"`
	AuthToken             string `env:"AME_AUTH_TOKEN"`
	ObjectStorageEndpoint string `env:"AME_OBJECT_STORAGE_ENDPOINT"`
	BucketName            string `env:"AME_BUCKET"`
}

// getPathToTestEnv searches recursively upwards from dir until it finds the file described in
// testenvName. This file will contain the environment variables needed to generate a TestEnvConfig.
// If any errors are encounted they are logged and the function panics, as this is only meant to be
// used during test setup.
// The complete path to the file is then returned.
func getPathToTestEnv(dir string) string {
	entries, err := os.ReadDir(dir)
	if err != nil {
		log.Fatal(err)
	}

	for _, e := range entries {
		if e.Name() == testenvName {
			return path.Join(dir, testenvName)
		}
	}

	return getPathToTestEnv(path.Join(dir, ".."))
}

// TestEnv generates a TestEnvConfig based on the environment variables specified in the
// test.env file in the root of this project. If that file is missing check the README
// for how generate it.
func TestEnv() TestEnvConfig {
	// It is important to use the Overload method here, as we never want to
	// keep any existing environment variables with matching keys. The goal is
	// to use the values in the .env file, as it should have the latest values
	// based on the test cluster.
	err := godotenv.Overload(getPathToTestEnv("."))
	if err != nil {
		log.Fatal(err)
	}

	var cfg TestEnvConfig
	_, err = env.UnmarshalFromEnviron(&cfg)
	if err != nil {
		log.Fatal(err)
	}

	return cfg
}
