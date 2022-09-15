package ameproject

import (
	"context"
	"fmt"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"teainspace.com/ame/internal/config"
	task "teainspace.com/ame/server/grpc"
)

// TODO: can we use this error when calling ReadProjectFile?
type MissingProjectFileErr struct {
	dir string
	Err error
}

func (e MissingProjectFileErr) Error() string {
	return fmt.Sprintf("failed to find project file %s in %s", AmeProjectFileName, e.dir)
}

func (e MissingProjectFileErr) Unwrap() error {
	return e.Err
}

func NewMissingProjectFile(dir string, err error) MissingProjectFileErr {
	return MissingProjectFileErr{
		dir: dir,
		Err: err,
	}
}

func ProjectFromWd(ctx context.Context) (Project, error) {
	cliCfg, err := config.GenCliConfig()
	if err != nil {
		return Project{}, err
	}

	projectFile, err := ReadProjectFile(".")
	if err != nil {
		return Project{}, err
	}

	pCfg := ProjectConfig{
		Directory:   ".",
		Name:        projectFile.ProjectName,
		AuthToken:   cliCfg.AuthToken,
		ProjectFile: projectFile,
	}

	var opts []grpc.DialOption
	opts = append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))

	conn, err := grpc.Dial(cliCfg.AmeEndpoint, opts...)
	if err != nil {
		return Project{}, err
	}

	taskClient := task.NewTaskServiceClient(conn)

	return NewProject(pCfg, taskClient), nil
}
