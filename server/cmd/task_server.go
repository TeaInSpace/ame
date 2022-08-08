package task

import (
	"bytes"
	"context"
	fmt "fmt"
	io "io"
	"net"
	"os"

	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	_ "github.com/joho/godotenv/autoload"
	"google.golang.org/grpc"
	"google.golang.org/grpc/metadata"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/cmd/ame/filescanner"
	clientset "teainspace.com/ame/generated/clientset/versioned"
	"teainspace.com/ame/server/storage"
)

const (
	MdKeyProjectName                          = "project-name"
	taskServerEnvKeyObjectStorageBucketName   = "TASK_SERVER_OBJECT_STORAGE_BUCKET_NAME"
	taskServerEnvKeyObjectStorageEndpoint     = "TASK_SERVER_OBJECT_STORAGE_ENDPOINT"
	taskServerEnvKeyObjectStorageAccessKey    = "TASK_SERVER_OBJECT_STORAGE_ENDPOINT_ACCESS_KEY"
	taskServerEnvKeyObjectStorageAccessSecret = "TASK_SERVER_OBJECT_STORAGE_ENDPOINT_ACCESS_SECRET"
)

type TaskServer struct {
	ameClientSet clientset.Interface
	fileStorage  storage.Storage
}

type TaskServerConfiguration struct {
	bucketName                string
	objectStorageEndpoint     string
	objectStorageAccessKey    string
	objectStorageAccessSecret string
	useHTTPSForObjectStorage  bool
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

	return TaskServerConfiguration{
		bucketName:                bucketName,
		objectStorageEndpoint:     objectStorageEndpoint,
		objectStorageAccessKey:    objectStorageAccessKey,
		objectStorageAccessSecret: objectStorageAccessSecret,
		useHTTPSForObjectStorage:  false,
	}, nil
}

func NewTaskServer(ctx context.Context, client clientset.Interface, cfg TaskServerConfiguration) (TaskServer, error) {
	s3Client, err := storage.CreateS3Client(ctx, cfg.objectStorageEndpoint, "us-east-1", func(opts *s3.Options) {
		opts.EndpointOptions.DisableHTTPS = !cfg.useHTTPSForObjectStorage
		opts.Credentials = credentials.NewStaticCredentialsProvider(cfg.objectStorageAccessKey, cfg.objectStorageAccessSecret, "")
		fmt.Println("using credentials", cfg.objectStorageAccessKey, cfg.objectStorageAccessSecret)
	})
	if err != nil {
		return TaskServer{}, err
	}

	return TaskServer{client, storage.NewS3Storage(*s3Client, cfg.bucketName)}, nil
}

func (s *TaskServer) InitStorage(ctx context.Context) error {
	return s.fileStorage.PrepareStorage(ctx)
}

func Run(ctx context.Context, cfg *rest.Config, port string) (net.Listener, func() error, error) {
	listener, err := net.Listen("tcp", fmt.Sprintf(":%s", port))
	if err != nil {
		return listener, func() error { return nil }, err
	}

	var opts []grpc.ServerOption
	grpcServer := grpc.NewServer(opts...)
	taskServerConfig, err := TaskServerConfigFromEnv()
	if err != nil {
		return listener, func() error { return nil }, err
	}

	fmt.Println("Using config: ", taskServerConfig)

	taskServer, err := NewTaskServer(ctx, clientset.NewForConfigOrDie(cfg), taskServerConfig)
	if err != nil {
		return listener, func() error { return nil }, err
	}

	err = taskServer.InitStorage(ctx)
	if err != nil {
		return listener, func() error { return nil }, err
	}

	RegisterTaskServiceServer(grpcServer, taskServer)
	RegisterHealthServer(grpcServer, taskServer)

	return listener, func() error {
		return grpcServer.Serve(listener)
	}, nil
}

func (s TaskServer) CreateTask(ctx context.Context, taskCreateRequest *TaskCreateRequest) (*v1alpha1.Task, error) {
	opts := v1.CreateOptions{}
	if taskCreateRequest.CreateOptions != nil {
		opts = *taskCreateRequest.GetCreateOptions()
	}
	return s.ameClientSet.AmeV1alpha1().Tasks(taskCreateRequest.GetNamespace()).Create(ctx, taskCreateRequest.Task, opts)
}

func (s TaskServer) GetTask(ctx context.Context, taskGetRequest *TaskGetRequest) (*v1alpha1.Task, error) {
	opts := v1.GetOptions{}
	if taskGetRequest.GetOptions != nil {
		opts = *taskGetRequest.GetGetOptions()
	}

	return s.ameClientSet.AmeV1alpha1().Tasks(taskGetRequest.GetNamespace()).Get(ctx, taskGetRequest.GetName(), opts)
}

func (s TaskServer) Check(context.Context, *HealthCheckRequest) (*HealthCheckResponse, error) {
	return &HealthCheckResponse{
		Status: HealthCheckResponse_SERVING,
	}, nil
}

func (s TaskServer) FileUpload(fileUploadServer TaskService_FileUploadServer) error {
	data := []byte{}
	for {
		f, err := fileUploadServer.Recv()

		if err == io.EOF {
			err = s.uploadReceivedFiles(fileUploadServer.Context(), data)
			if err != nil {
				errFromClose := fileUploadServer.SendAndClose(&UploadStatus{Status: UploadStatus_FAILURE})

				if errFromClose != nil {
					return errFromClose
				}

				return err
			}

			err = fileUploadServer.SendAndClose(&UploadStatus{Status: UploadStatus_SUCCESS})
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
		return fmt.Errorf("Could not get metadata from incoming stream.")
	}

	vals := md.Get(MdKeyProjectName)
	if len(vals) != 1 {
		return fmt.Errorf("Expect to get one project name in metdata instead got %s", vals)
	}

	return filescanner.NewTarProjectPacker(vals[0]).WalkProject(bytes.NewBuffer(data), func(p storage.ProjectFile) error {
		return s.fileStorage.StoreFile(ctx, p)
	})
}

func (s TaskServer) Watch(*HealthCheckRequest, Health_WatchServer) error {
	return nil
}
