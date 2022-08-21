package task

import (
	grpc "google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// PrepareTaskClient creates a TaskServiceClient with the insecure transport credentials transport options enabled.
// This is intended to be used when ssl is not handled at a gRPC application level.
func PrepareTaskClient(endpoint string, opts ...grpc.DialOption) (TaskServiceClient, error) {
	conn, err := grpc.Dial(endpoint, append(opts, grpc.WithTransportCredentials(insecure.NewCredentials()))...)
	if err != nil {
		return nil, err
	}

	return NewTaskServiceClient(conn), nil
}
