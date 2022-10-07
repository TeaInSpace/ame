package task

import (
	"bytes"
	"context"
	fmt "fmt"
	io "io"
	"net"
	"os"
	"time"

	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	grpc_middleware "github.com/grpc-ecosystem/go-grpc-middleware"
	grpc_auth "github.com/grpc-ecosystem/go-grpc-middleware/auth"
	grpc_zap "github.com/grpc-ecosystem/go-grpc-middleware/logging/zap"
	_ "github.com/joho/godotenv/autoload"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/metadata"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/cmd/ame/filescanner"
	clientset "teainspace.com/ame/generated/clientset/versioned"
	"teainspace.com/ame/internal/auth"
	task_service "teainspace.com/ame/server/grpc"
	"teainspace.com/ame/server/logs"
	"teainspace.com/ame/server/storage"
)

const (
	MdKeyProjectName                          = "project-name"
	taskServerEnvKeyObjectStorageBucketName   = "TASK_SERVER_OBJECT_STORAGE_BUCKET_NAME"
	taskServerEnvKeyObjectStorageEndpoint     = "TASK_SERVER_OBJECT_STORAGE_ENDPOINT"
	taskServerEnvKeyObjectStorageAccessKey    = "TASK_SERVER_OBJECT_STORAGE_ENDPOINT_ACCESS_KEY"
	taskServerEnvKeyObjectStorageAccessSecret = "TASK_SERVER_OBJECT_STORAGE_ENDPOINT_ACCESS_SECRET"
	taskServerEnvKeyTargetNamespace           = "TASK_SERVER_TARGET_NAMESPACE"
)

type TaskServer struct {
	ameClientSet    clientset.Interface
	fileStorage     storage.Storage
	restCfg         *rest.Config
	targetNamespace string
}

type TaskServerConfiguration struct {
	bucketName                string
	objectStorageEndpoint     string
	objectStorageAccessKey    string
	objectStorageAccessSecret string
	useHTTPSForObjectStorage  bool
	TargetNamespace           string
}

type MissingEnvVariableError struct {
	envVarKey string
}

func (e *MissingEnvVariableError) Error() string {
	return fmt.Sprintf("Missing environment variable %s", e.envVarKey)
}

func NewMissingEnvVarError(key string) *MissingEnvVariableError {
	return &MissingEnvVariableError{key}
}

func TaskServerConfigFromEnv() (TaskServerConfiguration, error) {
	bucketName := os.Getenv(taskServerEnvKeyObjectStorageBucketName)
	if bucketName == "" {
		return TaskServerConfiguration{}, NewMissingEnvVarError(taskServerEnvKeyObjectStorageBucketName)
	}

	objectStorageEndpoint := os.Getenv(taskServerEnvKeyObjectStorageEndpoint)
	if objectStorageEndpoint == "" {
		return TaskServerConfiguration{}, NewMissingEnvVarError(taskServerEnvKeyObjectStorageEndpoint)
	}

	objectStorageAccessKey := os.Getenv(taskServerEnvKeyObjectStorageAccessKey)
	if objectStorageAccessKey == "" {
		return TaskServerConfiguration{}, NewMissingEnvVarError(taskServerEnvKeyObjectStorageAccessKey)
	}

	objectStorageAccessSecret := os.Getenv(taskServerEnvKeyObjectStorageAccessSecret)
	if objectStorageAccessSecret == "" {
		return TaskServerConfiguration{}, NewMissingEnvVarError(taskServerEnvKeyObjectStorageAccessSecret)
	}

	targetNamespace := os.Getenv(taskServerEnvKeyTargetNamespace)
	if targetNamespace == "" {
		return TaskServerConfiguration{}, NewMissingEnvVarError(taskServerEnvKeyTargetNamespace)
	}

	return TaskServerConfiguration{
		bucketName:                bucketName,
		objectStorageEndpoint:     objectStorageEndpoint,
		objectStorageAccessKey:    objectStorageAccessKey,
		objectStorageAccessSecret: objectStorageAccessSecret,
		useHTTPSForObjectStorage:  false,
		TargetNamespace:           targetNamespace,
	}, nil
}

func NewTaskServer(ctx context.Context, client clientset.Interface, cfg TaskServerConfiguration, restCfg *rest.Config) (TaskServer, error) {
	s3Client, err := storage.CreateS3Client(ctx, cfg.objectStorageEndpoint, "us-east-1", func(opts *s3.Options) {
		opts.EndpointOptions.DisableHTTPS = !cfg.useHTTPSForObjectStorage
		opts.Credentials = credentials.NewStaticCredentialsProvider(cfg.objectStorageAccessKey, cfg.objectStorageAccessSecret, "")
		fmt.Println("using credentials", cfg.objectStorageAccessKey, cfg.objectStorageAccessSecret)
	})
	if err != nil {
		return TaskServer{}, err
	}

	return TaskServer{client, storage.NewS3Storage(*s3Client, cfg.bucketName), restCfg, cfg.TargetNamespace}, nil
}

func (s *TaskServer) InitStorage(ctx context.Context) error {
	return s.fileStorage.PrepareStorage(ctx)
}

func Run(ctx context.Context, cfg *rest.Config, port string) (net.Listener, func() error, error) {
	logger, err := zap.NewProduction()
	if err != nil {
		return nil, nil, err
	}

	listener, err := net.Listen("tcp", fmt.Sprintf(":%s", port))
	if err != nil {
		return listener, func() error { return nil }, err
	}

	authenticator, err := auth.EnvAuthenticator()
	if err != nil {
		return listener, func() error { return nil }, err
	}
	grpcServer := grpc.NewServer(grpc.StreamInterceptor(
		grpc_middleware.ChainStreamServer(grpc_auth.StreamServerInterceptor(authenticator),
			grpc_zap.StreamServerInterceptor(logger),
		)),
		grpc.UnaryInterceptor(grpc_middleware.ChainUnaryServer(
			grpc_zap.UnaryServerInterceptor(logger),
		)),
	)
	taskServerConfig, err := TaskServerConfigFromEnv()
	if err != nil {
		return listener, func() error { return nil }, err
	}

	fmt.Println("Using config: ", taskServerConfig)

	taskServer, err := NewTaskServer(ctx, clientset.NewForConfigOrDie(cfg), taskServerConfig, cfg)
	if err != nil {
		return listener, func() error { return nil }, err
	}

	err = taskServer.InitStorage(ctx)
	if err != nil {
		return listener, func() error { return nil }, err
	}

	task_service.RegisterTaskServiceServer(grpcServer, taskServer)
	task_service.RegisterHealthServer(grpcServer, taskServer)

	return listener, func() error {
		return grpcServer.Serve(listener)
	}, nil
}

func (s TaskServer) CreateTask(ctx context.Context, taskCreateRequest *task_service.TaskCreateRequest) (*v1alpha1.Task, error) {
	opts := v1.CreateOptions{}
	if taskCreateRequest.CreateOptions != nil {
		opts = *taskCreateRequest.GetCreateOptions()
	}
	return s.ameClientSet.AmeV1alpha1().Tasks(s.targetNamespace).Create(ctx, taskCreateRequest.Task, opts)
}

func (s TaskServer) GetTask(ctx context.Context, taskGetRequest *task_service.TaskGetRequest) (*v1alpha1.Task, error) {
	opts := v1.GetOptions{}
	if taskGetRequest.GetOptions != nil {
		opts = *taskGetRequest.GetGetOptions()
	}

	return s.ameClientSet.AmeV1alpha1().Tasks(taskGetRequest.GetNamespace()).Get(ctx, taskGetRequest.GetName(), opts)
}

func (s TaskServer) Check(context.Context, *task_service.HealthCheckRequest) (*task_service.HealthCheckResponse, error) {
	return &task_service.HealthCheckResponse{
		Status: task_service.HealthCheckResponse_SERVING,
	}, nil
}

func (s TaskServer) FileUpload(fileUploadServer task_service.TaskService_FileUploadServer) error {
	data := []byte{}
	for {
		f, err := fileUploadServer.Recv()

		if err == io.EOF {
			err = s.uploadReceivedFiles(fileUploadServer.Context(), data)
			if err != nil {
				errFromClose := fileUploadServer.SendAndClose(&task_service.UploadStatus{Status: task_service.UploadStatus_FAILURE})

				if errFromClose != nil {
					return errFromClose
				}

				return err
			}

			err = fileUploadServer.SendAndClose(&task_service.UploadStatus{Status: task_service.UploadStatus_SUCCESS})
			if err != nil {
				return err
			}
			return nil
		}

		if err != nil {
			return err
		}

		data = append(data, f.Contents...)
	}
}

func (s TaskServer) uploadReceivedFiles(ctx context.Context, data []byte) error {
	md, ok := metadata.FromIncomingContext(ctx)
	if !ok {
		return fmt.Errorf("could not get metadata from incoming stream")
	}

	vals := md.Get(MdKeyProjectName)
	if len(vals) != 1 {
		return fmt.Errorf("expect to get one project name in metdata instead got %s", vals)
	}

	return filescanner.NewTarProjectPacker(vals[0]).WalkProject(bytes.NewBuffer(data), func(p storage.ProjectFile) error {
		return s.fileStorage.StoreFileInProject(ctx, vals[0], p)
	})
}

func (s TaskServer) Watch(*task_service.HealthCheckRequest, task_service.Health_WatchServer) error {
	return nil
}

func (s TaskServer) GetLogs(req *task_service.GetLogsRequest, server task_service.TaskService_GetLogsServer) error {
	return logs.StreamTaskLogs(server.Context(), logs.StreamConfig{
		Follow:        req.Follow,
		StreamAllLogs: req.GetAllLogs,
		Task:          req.Task,
		Timeout:       time.Second * 60,
		Sender: func(tle logs.TaskLogEntry) error {
			return server.Send(&task_service.LogEntry{
				Content: string(tle),
			})
		},
	}, s.restCfg)
}

func (s TaskServer) GetArtifacts(req *task_service.ArtifactGetRequest, server task_service.TaskService_GetArtifactsServer) error {
	// TODO: Note that downloaded all of the artifacts at once is problematic for large artifacts
	// as the server will be loading them into memory all at once. When time permits we improce
	// how artifacts are transferred so they are not all loaded into the server's memory at once.
	artifacts, err := s.fileStorage.DownloadArtifacts(server.Context(), req.TaskName)
	if err != nil {
		return err
	}

	artBuf, err := filescanner.TarFiles(artifacts)
	if err != nil {
		return err
	}

	send := func(data []byte) error {
		return server.Send(&task_service.Chunk{
			Contents: data,
		})
	}

	return task_service.ProcessInChunks(artBuf, send, task_service.ChunkSize)
}

func (s TaskServer) CreateRecurringTask(ctx context.Context, req *task_service.RecurringTaskCreateRequest) (*v1alpha1.ReccurringTask, error) {
	opts := v1.CreateOptions{}
	if req.CreateOptions != nil {
		opts = *req.GetCreateOptions()
	}
	return s.ameClientSet.AmeV1alpha1().ReccurringTasks(s.targetNamespace).Create(ctx, req.GetTask(), opts)
}
