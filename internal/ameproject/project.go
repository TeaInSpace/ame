package ameproject

import (
	"archive/tar"
	"bytes"
	"context"
	"errors"
	"io"
	"path/filepath"

	"google.golang.org/grpc/metadata"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/cmd/ame/filescanner"
	genv1alpha "teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"

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
	Name      string
	Directory string
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

func NewProjectForDir(dir string, taskClient task_service.TaskServiceClient) Project {
	return NewProject(ProjectConfig{
		Name:      ProjectNameFromDir(dir),
		Directory: dir,
	}, taskClient)
}

func (p *Project) UploadAndRun(ctx context.Context, t *v1alpha1.Task) (*v1alpha1.Task, error) {
	err := p.UploadProject(ctx)
	if err != nil {
		return nil, err
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
	t, err := filescanner.TarDirectory(p.Directory, []string{})
	if err != nil {
		return err
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
		return err
	}

	_, err = uploadClient.CloseAndRecv()
	return err
}

type LogProcessor func(*task_service.LogEntry) error

func (p *Project) ProcessTaskLogs(ctx context.Context, targetTask *v1alpha1.Task, logProcessor LogProcessor) error {
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
