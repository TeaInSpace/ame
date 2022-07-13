package task

import (
	"context"
	fmt "fmt"
	"net"

	"google.golang.org/grpc"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/rest"
	"teainspace.com/ame/api/v1alpha1"

	clientset "teainspace.com/ame/generated/clientset/versioned"
)

type TaskServer struct {
	ameClientSet clientset.Interface
}

func NewTaskServer(client clientset.Interface) TaskServer {
	return TaskServer{client}
}

func Run(cfg *rest.Config, port int) (net.Listener, func() error, error) {
	listener, err := net.Listen("tcp", fmt.Sprintf("localhost:%d", port))
	if err != nil {
		return listener, func() error { return nil }, err
	}

	var opts []grpc.ServerOption
	grpcServer := grpc.NewServer(opts...)
	RegisterTaskServiceServer(grpcServer, NewTaskServer(clientset.NewForConfigOrDie(cfg)))

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
