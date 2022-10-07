package ameproject

import (
	"archive/tar"
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"path/filepath"

	"google.golang.org/grpc/metadata"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/cmd/ame/filescanner"
	genv1alpha "teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/auth"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	task_service "teainspace.com/ame/server/grpc"
	"teainspace.com/ame/server/storage"
)

// TODO: centralize the chunksize configuration
const MdKeyProjectName = "project-name"

func ProjectNameFromDir(dir string) string {
	return filepath.Base(dir)
}

type ProjectConfig struct {
	Name        string
	Directory   string
	AuthToken   string
	ProjectFile *ProjectFileCfg
}

type Project struct {
	ProjectConfig
	taskClient task_service.TaskServiceClient
}

func NewProject(cfg ProjectConfig, taskClient task_service.TaskServiceClient) Project {
	return Project{
		ProjectConfig: cfg,
		taskClient:    taskClient,
	}
}

func (p *Project) GetTaskSpec(name string) (*v1alpha1.TaskSpec, error) {
	for n := range p.ProjectFile.Specs {
		if TaskSpecName(name) == n {
			return p.ProjectFile.Specs[n], nil
		}
	}

	return nil, fmt.Errorf("could not find task specification for %s", name)
}

func (p *Project) UploadAndRunSpec(ctx context.Context, name string) (*v1alpha1.Task, error) {
	spec, err := p.GetTaskSpec(name)
	if err != nil {
		return nil, err
	}

	task := v1alpha1.NewTaskFromSpec(spec, name)

	return p.UploadAndRun(ctx, task)
}

func (p *Project) UploadAndRun(ctx context.Context, t *v1alpha1.Task) (*v1alpha1.Task, error) {
	ctx = auth.AuthorarizeCtx(ctx, p.AuthToken)
	err := p.UploadProject(ctx)
	if err != nil {
		return nil, fmt.Errorf("got error while attempting to upload project: %v", err)
	}

	taskInCluster, err := p.UploadTaskForProject(ctx, t)
	if err != nil {
		return nil, err
	}

	return taskInCluster, nil
}

func (p *Project) UploadTaskForProject(ctx context.Context, t *v1alpha1.Task) (*v1alpha1.Task, error) {
	return p.taskClient.CreateTask(ctx,
		&task_service.TaskCreateRequest{
			Task: t,
		})
}

func (p *Project) UploadProject(ctx context.Context) error {
	// There is no need to upload the ame project file, so it is filtered out.
	t, err := filescanner.TarDirectory(p.Directory, []string{AmeProjectFileName})
	if err != nil {
		return fmt.Errorf("got error while attempting to upload project: %v", err)
	}

	ctx = metadata.AppendToOutgoingContext(ctx, MdKeyProjectName, p.Name)

	uploadClient, err := p.taskClient.FileUpload(ctx)
	if err != nil {
		return err
	}

	send := func(data []byte) error {
		return uploadClient.Send(&task_service.Chunk{
			Contents: data,
		})
	}

	err = task_service.ProcessInChunks(t, send, task_service.ChunkSize)
	if err != nil {
		return fmt.Errorf("got error while transferring chunsk: %v", err)
	}

	_, err = uploadClient.CloseAndRecv()
	return err
}

type LogProcessor func(*task_service.LogEntry) error

func (p *Project) ProcessTaskLogs(ctx context.Context, targetTask *v1alpha1.Task, logProcessor LogProcessor) error {
	ctx = auth.AuthorarizeCtx(ctx, p.AuthToken)
	logsClient, err := p.taskClient.GetLogs(ctx, &task_service.GetLogsRequest{
		Task:       targetTask,
		Follow:     true,
		GetAllLogs: true,
	})
	if err != nil {
		return err
	}

	// TODO determine why this loop is necessary
	for {
		logEntry, err := logsClient.Recv()
		if errors.Is(err, io.EOF) {
			break
		}

		if err != nil {
			return err
		}

		err = logProcessor(logEntry)
		if err != nil {
			return err
		}
	}

	return nil
}

func (p *Project) GetTask(ctx context.Context, taskName string, ns string) (*v1alpha1.Task, error) {
	ctx = auth.AuthorarizeCtx(ctx, p.AuthToken)
	return p.taskClient.GetTask(ctx, &task_service.TaskGetRequest{
		Name:      taskName,
		Namespace: ns,
	})
}

func (p *Project) GetArtifacts(ctx context.Context, taskName string) ([]storage.ProjectFile, error) {
	ctx = auth.AuthorarizeCtx(ctx, p.AuthToken)
	return GetArtifacts(ctx, p.taskClient, taskName)
}

func (p *Project) ScheduleTask(ctx context.Context, spec v1alpha1.TaskSpec, schedule string) (*v1alpha1.ReccurringTask, error) {
	ctx = auth.AuthorarizeCtx(ctx, p.AuthToken)
	return p.taskClient.CreateRecurringTask(ctx, &task_service.RecurringTaskCreateRequest{
		Task: v1alpha1.NewRecurringTask(p.Name, spec, schedule),
	})
}

// TODO: GetArtifacts loads all of the artifacts into memory at once, this will not work for large artifacts.

func GetArtifacts(ctx context.Context, taskClient task_service.TaskServiceClient, taskName string) ([]storage.ProjectFile, error) {
	artifactsClient, err := taskClient.GetArtifacts(ctx, &task_service.ArtifactGetRequest{TaskName: taskName})
	if err != nil {
		return nil, err
	}

	data := []byte{}

	for {
		chunk, err := artifactsClient.Recv()
		if errors.Is(err, io.EOF) {
			break
		}

		if err != nil {
			return nil, err
		}

		data = append(data, chunk.Contents...)
	}

	files := []storage.ProjectFile{}
	err = filescanner.ReadFromTar(bytes.NewBuffer(data), func(h *tar.Header, b []byte) error {
		files = append(files, storage.ProjectFile{
			Data: b,
			Path: h.Name,
		})

		return nil
	})

	if err != nil {
		return nil, err
	}

	return files, nil
}

func GetTasksForProject(ctx context.Context, tasks genv1alpha.TaskInterface, projectId string) ([]v1alpha1.Task, error) {
	taskList, err := tasks.List(ctx, metav1.ListOptions{})
	if err != nil {
		return nil, err
	}

	var projectTasks []v1alpha1.Task
	for _, t := range taskList.Items {
		if t.Spec.ProjectId == projectId {
			projectTasks = append(projectTasks, t)
		}
	}

	return projectTasks, nil
}
