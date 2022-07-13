package task

import (
	context "context"
	"os"
	"testing"

	"github.com/brianvoe/gofakeit/v6"
	"github.com/stretchr/testify/assert"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	schema "k8s.io/apimachinery/pkg/runtime/schema"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/generated/clientset/versioned/fake"
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

func GenerateTaskServer(task amev1alpha1.Task) (TaskServer, *fake.Clientset) {
	fakeClient := fake.NewSimpleClientset(&task)
	return NewTaskServer(fakeClient), fakeClient
}

func TestCreateTask(t *testing.T) {
	ctx := context.Background()
	taskServer, fakeClient := GenerateTaskServer(generateRandomTask())
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
	taskServer, _ := GenerateTaskServer(randomTask)
	extractedTask, err := taskServer.GetTask(ctx, &TaskGetRequest{Namespace: testNamespace, Name: randomTask.GetName()})
	assert.NoError(t, err)
	assert.NotNil(t, *extractedTask)
	assert.Equal(t, randomTask, *extractedTask)
}
