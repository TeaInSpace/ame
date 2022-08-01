package task

import (
	context "context"
	io "io"
	math "math"
	"os"
	"testing"
	"time"

	"github.com/brianvoe/gofakeit/v6"
	"github.com/stretchr/testify/assert"
	"google.golang.org/grpc/metadata"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	schema "k8s.io/apimachinery/pkg/runtime/schema"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/generated/clientset/versioned/fake"
)

const (
	objectStorageEndpoint = "http://127.0.0.1:9000"
	testBucketName        = "testbucket"
)

var (
	tasksResource = schema.GroupVersionResource{Group: "ame.teainspace.com", Version: "v1alpha1", Resource: "tasks"}
	testNamespace string
)

func TestMain(m *testing.M) {
	// Generate a random namespace to ensure that
	testNamespace = gofakeit.FirstName()
	os.Exit(m.Run())
}

func generateRandomTask() amev1alpha1.Task {
	return amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{Name: gofakeit.FirstName(), Namespace: testNamespace},
		TypeMeta:   v1.TypeMeta{APIVersion: "ame/v1alpha1", Kind: "Task"},
	}
}

func GenerateTaskServer(ctx context.Context, task amev1alpha1.Task) (TaskServer, *fake.Clientset, error) {
	fakeClient := fake.NewSimpleClientset(&task)
	serverCfg, err := TaskServerConfigFromEnv()
	if err != nil {
		return TaskServer{}, nil, err
	}

	serverCfg.bucketName = testBucketName
	serverCfg.objectStorageEndpoint = objectStorageEndpoint
	server, err := NewTaskServer(ctx, fakeClient, serverCfg)
	if err != nil {
		return TaskServer{}, nil, err
	}
	return server, fakeClient, nil
}

func TestCreateTask(t *testing.T) {
	ctx := context.Background()
	taskServer, fakeClient, err := GenerateTaskServer(ctx, generateRandomTask())
	assert.NoError(t, err)
	testTask, err := taskServer.CreateTask(ctx, &TaskCreateRequest{Namespace: testNamespace, Task: &amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{Name: "test123", Namespace: testNamespace},
		TypeMeta:   v1.TypeMeta{APIVersion: "ame/v1alpha1", Kind: "Task"},
	}})
	assert.NoError(t, err)
	assert.NotNil(t, testTask)

	trackedTask, err := fakeClient.Tracker().Get(tasksResource, testNamespace, testTask.GetName())
	assert.NoError(t, err)
	assert.EqualValues(t, testTask, trackedTask)
}

func TestGetTask(t *testing.T) {
	ctx := context.Background()
	randomTask := generateRandomTask()
	taskServer, _, err := GenerateTaskServer(ctx, randomTask)
	assert.NoError(t, err)
	extractedTask, err := taskServer.GetTask(ctx, &TaskGetRequest{Namespace: testNamespace, Name: randomTask.GetName()})
	assert.NoError(t, err)
	assert.NotNil(t, *extractedTask)
	assert.Equal(t, randomTask, *extractedTask)
}

func TestFileUpload(t *testing.T) {
	projectName := "myproject"
	ctx := context.Background()

	ctx = metadata.AppendToOutgoingContext(ctx, mdKeyProjectName, projectName)
	taskServer, _, err := GenerateTaskServer(ctx, amev1alpha1.Task{})
	assert.NoError(t, err)

	err = taskServer.fileStorage.PrepareStorage(ctx)
	assert.NoError(t, err)

	fileChan := make(chan []byte)
	uploadSt := UploadStatus{}
	mockStream := NewMockFileUploadStream(ctx, fileChan, &uploadSt)
	go func() {
		err := taskServer.FileUpload(&mockStream)
		assert.NoError(t, err)
	}()

	fileData := "this is my data it will be split into multiple chunks"
	dataBytes := []byte(fileData)
	chunks := [][]byte{}
	nChunks := 10
	chunkLength := len(dataBytes) / nChunks
	for i := 0; i < nChunks+1; i++ {
		chunks = append(chunks, dataBytes[i*chunkLength:int(math.Min(float64((i*chunkLength)+chunkLength), float64(len(dataBytes))))])
	}

	for _, c := range chunks {
		fileChan <- c
	}

	close(fileChan)
	assert.Eventually(t, func() bool {
		return uploadSt.GetStatus() == UploadStatus_SUCCESS
	}, time.Second, 10*time.Millisecond)
	files, err := taskServer.fileStorage.DownloadFiles(ctx, projectName)
	assert.NoError(t, err)
	assert.Len(t, files, 1)
	assert.Equal(t, string(dataBytes), string(files[0].Data))
	err = taskServer.fileStorage.ClearStorage(ctx)
	assert.NoError(t, err)
}

type MockFileUploadStream struct {
	ctx      context.Context
	fileChan chan []byte
	uploadSt *UploadStatus
}

func NewMockFileUploadStream(ctx context.Context, fileChan chan []byte, uploadSt *UploadStatus) MockFileUploadStream {
	return MockFileUploadStream{ctx, fileChan, uploadSt}
}

func (s *MockFileUploadStream) Recv() (*Chunk, error) {
	chunk, chanOpen := <-s.fileChan
	if !chanOpen {
		return nil, io.EOF
	}

	return &Chunk{
		Contents: chunk,
	}, nil
}

func (s *MockFileUploadStream) SendAndClose(uploadSt *UploadStatus) error {
	*s.uploadSt = *uploadSt
	return nil
}

func (s *MockFileUploadStream) SetHeader(md metadata.MD) error {
	return nil
}

func (s *MockFileUploadStream) SetTrailer(md metadata.MD) {
}

func (s *MockFileUploadStream) SendHeader(md metadata.MD) error {
	return nil
}

func (s *MockFileUploadStream) Context() context.Context {
	return s.ctx
}

func (s *MockFileUploadStream) SendMsg(m interface{}) error {
	return nil
}

func (s *MockFileUploadStream) RecvMsg(m interface{}) error {
	return nil
}
