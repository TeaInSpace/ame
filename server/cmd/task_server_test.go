package task

import (
	context "context"
	io "io"
	"log"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/brianvoe/gofakeit/v6"
	"github.com/google/go-cmp/cmp"
	"github.com/google/go-cmp/cmp/cmpopts"
	"github.com/stretchr/testify/assert"
	"google.golang.org/grpc/metadata"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	schema "k8s.io/apimachinery/pkg/runtime/schema"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/cmd/ame/filescanner"
	"teainspace.com/ame/generated/clientset/versioned/fake"
	"teainspace.com/ame/internal/ameproject"
	"teainspace.com/ame/internal/auth"
	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/internal/testcfg"
	"teainspace.com/ame/internal/testdata"
	testenv "teainspace.com/ame/internal/testenv"
	task_service "teainspace.com/ame/server/grpc"
	"teainspace.com/ame/server/storage"
)

var (
	testCfg       testcfg.TestEnvConfig
	tasksResource = schema.GroupVersionResource{Group: "ame.teainspace.com", Version: "v1alpha1", Resource: "tasks"}
	taskClient    task_service.TaskServiceClient
	ctx           context.Context
)

func TestMain(m *testing.M) {
	// Generate a random namespace to ensure that
	testCfg = testcfg.TestEnv()

	var err error
	taskClient, err = task_service.PrepareTaskClient(testCfg.AmeServerEndpoint)
	if err != nil {
		log.Fatal(err)
	}

	ctx = context.Background()

	os.Exit(m.Run())
}

func generateRandomTask() amev1alpha1.Task {
	return amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{Name: gofakeit.FirstName(), Namespace: testCfg.Namespace},
		TypeMeta:   v1.TypeMeta{APIVersion: "ame/v1alpha1", Kind: "Task"},
	}
}

func GenerateTaskServer(ctx context.Context, task amev1alpha1.Task) (TaskServer, *fake.Clientset, error) {
	fakeClient := fake.NewSimpleClientset(&task)
	serverCfg, err := TaskServerConfigFromEnv()
	if err != nil {
		return TaskServer{}, nil, err
	}

	serverCfg.bucketName = testCfg.BucketName
	serverCfg.objectStorageEndpoint = testCfg.ObjectStorageEndpoint
	restCfg, err := clients.KubeClientFromConfig()
	if err != nil {
		return TaskServer{}, nil, err
	}
	server, err := NewTaskServer(ctx, fakeClient, serverCfg, restCfg)
	if err != nil {
		return TaskServer{}, nil, err
	}
	return server, fakeClient, nil
}

func TestCreateTask(t *testing.T) {
	taskServer, fakeClient, err := GenerateTaskServer(ctx, generateRandomTask())
	assert.NoError(t, err)
	testTask, err := taskServer.CreateTask(ctx, &task_service.TaskCreateRequest{Namespace: testCfg.Namespace, Task: &amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{Name: "test123", Namespace: testCfg.Namespace},
		TypeMeta:   v1.TypeMeta{APIVersion: "ame/v1alpha1", Kind: "Task"},
	}})
	assert.NoError(t, err)
	assert.NotNil(t, testTask)

	trackedTask, err := fakeClient.Tracker().Get(tasksResource, testCfg.Namespace, testTask.GetName())
	assert.NoError(t, err)
	assert.EqualValues(t, testTask, trackedTask)
}

func TestGetLogs(t *testing.T) {
	_, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	expectedLogEntry := "somelog"
	p := ameproject.NewProject(ameproject.ProjectConfig{
		Name:      ameproject.ProjectNameFromDir(testenv.EchoProjectDir),
		Directory: testenv.EchoProjectDir,
	},
		taskClient,
	)

	echoTask := amev1alpha1.NewTask("python echo.py "+expectedLogEntry, p.Name)

	ctx = auth.AuthorarizeCtx(ctx, testCfg.AuthToken)
	echoTask, err = p.UploadAndRun(ctx, echoTask)
	if err != nil {
		t.Fatal(err)
	}

	// Setting a timeout for the context ensures that logs are not streamed
	// indefinitely.
	ctx, cancelCtx := context.WithTimeout(ctx, time.Second*100)
	defer cancelCtx()

	var logs []*task_service.LogEntry
	err = p.ProcessTaskLogs(ctx, echoTask, func(le *task_service.LogEntry) error {
		logs = append(logs, le)
		return nil
	})

	if err != nil {
		t.Error(err)
	}

	foundLog := false
	for _, logEntry := range logs {
		if strings.Contains(logEntry.Content, expectedLogEntry) {
			foundLog = true
			break
		}
	}

	if !foundLog {
		t.Errorf("did not find expected log entry: %s, in logs: %v", expectedLogEntry, logs)
	}
}

func TestGetArtifacts(t *testing.T) {
	taskName := "mytasksdsads"
	store, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	err = store.StoreArtifacts(ctx, taskName, testdata.TestFiles)
	if err != nil {
		t.Fatal(err)
	}

	// Note that the project name is left empty here as it should not be required
	// to call GetArtifacts.
	ctx = auth.AuthorarizeCtx(ctx, testCfg.AuthToken)

	files, err := ameproject.GetArtifacts(ctx, taskClient, taskName)
	if err != nil {
		t.Error(err)
	}

	diff := cmp.Diff(testdata.TestFiles, files, cmpopts.SortSlices(storage.FileCmp))
	if diff != "" {
		t.Errorf("expected downloaded artifacts: %+v to match the uploaded artifacts: %+v, but got diff: %s", files, testdata.TestFiles, diff)
	}
}

func TestGetTask(t *testing.T) {
	randomTask := generateRandomTask()
	taskServer, _, err := GenerateTaskServer(ctx, randomTask)
	assert.NoError(t, err)
	extractedTask, err := taskServer.GetTask(ctx, &task_service.TaskGetRequest{Namespace: testCfg.Namespace, Name: randomTask.GetName()})
	assert.NoError(t, err)
	assert.NotNil(t, *extractedTask)
	assert.Equal(t, randomTask, *extractedTask)
}

func TestFileUpload(t *testing.T) {
	projectName := "myproject"
	ctx = metadata.NewIncomingContext(ctx, metadata.MD{MdKeyProjectName: []string{projectName}})
	taskServer, _, err := GenerateTaskServer(ctx, amev1alpha1.Task{})
	assert.NoError(t, err)

	err = taskServer.fileStorage.ClearStorage(ctx)
	if err != nil {
		t.Error(err)
	}
	err = taskServer.fileStorage.PrepareStorage(ctx)
	assert.NoError(t, err)

	fileChan := make(chan []byte)
	uploadSt := task_service.UploadStatus{}
	mockStream := NewMockFileUploadStream(ctx, fileChan, &uploadSt)
	go func() {
		err := taskServer.FileUpload(&mockStream)
		if err != nil && err != io.EOF {
			t.Error(err)
		}
	}()

	files := []storage.ProjectFile{
		{
			Path: "myfile",
			Data: []byte("this is my data it will be split into multiple chunks"),
		},
	}

	tempDir, err := dirtools.MkAndPopulateDirTemp("mydir", files)
	if err != nil {
		t.Fatal(err)
	}
	data, err := filescanner.TarDirectory(tempDir, []string{})
	if err != nil {
		t.Fatal(err)
	}

	nChunks := 0
	for {
		d := make([]byte, 5)
		_, err := data.Read(d)
		if err == io.EOF {
			break
		}
		nChunks += 1
		fileChan <- d
	}

	if nChunks <= 2 {
		// To properly test the file uploades we need to send
		// multiple chunks.
		t.Errorf("Should send >2 chunks, but sent %d", nChunks)
	}

	close(fileChan)
	assert.Eventually(t, func() bool {
		return uploadSt.GetStatus() == task_service.UploadStatus_SUCCESS
	}, time.Second, 10*time.Millisecond)
	uploadedFiles, err := taskServer.fileStorage.DownloadFiles(ctx, projectName)
	if err != nil {
		t.Error(err)
	}

	fileDiffs := dirtools.DiffFiles(files, uploadedFiles)
	if len(fileDiffs) > 0 {
		t.Errorf("Uploaded %v, expected %v, diffs: %v", uploadedFiles, files, fileDiffs)
	}
}

type MockFileUploadStream struct {
	ctx      context.Context
	fileChan chan []byte
	uploadSt *task_service.UploadStatus
}

func NewMockFileUploadStream(ctx context.Context, fileChan chan []byte, uploadSt *task_service.UploadStatus) MockFileUploadStream {
	return MockFileUploadStream{ctx, fileChan, uploadSt}
}

func (s *MockFileUploadStream) Recv() (*task_service.Chunk, error) {
	chunk, chanOpen := <-s.fileChan
	if !chanOpen {
		return nil, io.EOF
	}

	return &task_service.Chunk{
		Contents: chunk,
	}, nil
}

func (s *MockFileUploadStream) SendAndClose(uploadSt *task_service.UploadStatus) error {
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
