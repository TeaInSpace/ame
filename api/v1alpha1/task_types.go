/*
Copyright 2022.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package v1alpha1

import (
	"context"
	fmt "fmt"
	"time"

	"github.com/pkg/errors"
	v1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/labels"
	runtime "k8s.io/apimachinery/pkg/runtime"
	clientv1 "k8s.io/client-go/kubernetes/typed/core/v1"
)

// EDIT THIS FILE!  THIS IS SCAFFOLDING FOR YOU TO OWN!
// NOTE: json tags are required.  Any new fields you add must have json tags for the fields to be serialized.

// TaskSpec defines the desired state of Task
type TaskSpec struct {
	// The command AME will use to execute the Task.
	// The command must be runnable from a bash shell.
	// TODO: define propper requirements for the run command.
	RunCommand string `json:"runcommand,omitempty"`

	// A unique identifier for the project wich the Task will
	// be running based on.
	ProjectId string `json:"projectid,omitempty"`

	// Environment variables that will be made avaialbe to the Task during execution.
	Env []TaskEnvVar `json:"env,omitempty"`

	// Secrets that will be made available to the Task during execution.
	Secrets []TaskSecret `json:"secrets,omitempty"`

	// Pipeline defines a sequence of tasks to execute.
	// If a pipeline is specified the rest of the fields in this
	// specification are ignored.
	Pipeline []PipelineStep `json:"pipeline,omitempty"`
}

// A PipelineStep is essentially a duplicated of the TaskSpec, which represents each step in a pipeline.
// Due to limitations with the current tooling used for generated manifests from the types in this file,
// we can't have recursive types and therefore we need this duplicated for now. See issue 43.
type PipelineStep struct {
	// TaskName is the name of the task represented by this step.
	TaskName string `json:"taskname,omitempty"`

	// The command AME will use to execute the Task.
	// The command must be runnable from a bash shell.
	RunCommand string `json:"runcommand,omitempty"`

	// Environment variables that will be made avaialbe to the Task during execution.
	Env []TaskEnvVar `json:"env,omitempty"`

	// Secrets that will be made available to the Task during execution.
	Secrets []TaskSecret `json:"secrets,omitempty"`
}

// A TaskEnvVar represents an environment variable
// made available to a task during execution.
type TaskEnvVar struct {
	Name  string `json:"name"`
	Value string `json:"value"`
}

// A TaskSecret represnts a secret identified by the Name which
// will be injected into the Task's execution as an environment
// variable with EnvKey as the key.
type TaskSecret struct {
	Name   string `json:"name"`
	EnvKey string `json:"envkey"`
}

// TaskStatus defines the observed state of Task
type TaskStatus struct {
	// INSERT ADDITIONAL STATUS FIELD - define observed state of cluster
	// Important: Run "make" to regenerate code after modifying this file
}

//+genclient
//+kubebuilder:object:root=true
//+kubebuilder:subresource:status

// Task is the Schema for the tasks API
type Task struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   TaskSpec   `json:"spec,omitempty"`
	Status TaskStatus `json:"status,omitempty"`
}

//+kubebuilder:object:root=true

// TaskList contains a list of Task
type TaskList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []Task `json:"items"`
}

func init() {
	SchemeBuilder.Register(&Task{}, &TaskList{})
}

// TaskOwnerRef returns an OwnerReference referencing the given Task.
func TaskOwnerRef(scheme *runtime.Scheme, task *Task) (metav1.OwnerReference, error) {
	gvks, err := GenGvks(scheme, task)
	if len(gvks) == 0 {
		return metav1.OwnerReference{}, errors.Errorf("Could not find a GroupVersionKind for task: %s", task.GetName())
	}
	// TODO: Add support for multiple GroupVersionKinds.

	return GenOwnRef(task.ObjectMeta, gvks), err
}

// GetTaskPod finds the Pod executing a Task based on the label ame-task being set to the Task name.
// If multiple pods are found matching the label selector an error is returned that should not happen
// if AME is operating correctly.
// If no pod if found an error is returned aswell.
// If the function succeeds, the Pod is returned.
func GetTaskPod(ctx context.Context, pods clientv1.PodInterface, task *Task) (*v1.Pod, error) {
	selector, err := labels.Parse(fmt.Sprintf("ame-task=%s", task.GetName()))
	if err != nil {
		return nil, err
	}

	for {
		// Allow time for the cluster state to change between pod requests.
		time.Sleep(time.Millisecond * 50)

		// Note that the timeout is enforced by the context
		// deadline causing pods.List to return an error.
		podList, err := pods.List(ctx, metav1.ListOptions{
			LabelSelector: selector.String(),
		})
		if err != nil {
			return nil, fmt.Errorf("from getTaskPodWithinTimeout, could not find pod for task %s, with selector: %v, got err: %v", task.GetName(), selector, err)
		}

		// There is only every be a single pod per Task, if muliple Pods are encountered
		// something is very wrong.
		if len(podList.Items) > 1 {
			return nil, fmt.Errorf("expected 1 pod, got %d instead", len(podList.Items))
		}

		// Is no pods are found and the task is probably not running yet.
		if len(podList.Items) == 0 {
			continue
		}

		return &podList.Items[0], nil
	}
}

// NewTask creates a new Task object.
// The Task's GenerateName field is set using the projectId,
// this ensures that every task will have a unique name
// when created in a Kubernetes cluster.
// A pointer to the Task object is returned.
func NewTask(runCmd string, projectId string) *Task {
	return &Task{
		ObjectMeta: metav1.ObjectMeta{GenerateName: projectId},
		Spec: TaskSpec{
			RunCommand: runCmd,
			ProjectId:  projectId,
			Env:        []TaskEnvVar{},
			Secrets:    []TaskSecret{},
			Pipeline:   []PipelineStep{},
		},
	}
}

func NewTaskFromSpec(spec *TaskSpec, taskname string) *Task {
	return &Task{
		ObjectMeta: metav1.ObjectMeta{GenerateName: taskname},
		Spec:       *spec,
	}
}

// WfSpecFromPipelineStep generates a TaskSpec from a PipelineStep and an accompanying Task,
// is meant to be the parent task for that PipelineStep. The TaskSpec is then generated by copying
// the TaskSpec from the parent Task and then overriding the fields which are also present in the
// PipelineStep.
// A pointer to the generated TaskSpec is returned.
func WfSpecFromPipelineStep(parentTask *Task, step PipelineStep) *TaskSpec {
	// The deep copy is important here to avoid sharing any pointers with the
	// original Task struct.
	spec := parentTask.Spec.DeepCopy()
	spec.RunCommand = step.RunCommand
	spec.Pipeline = nil
	spec.Env = step.Env
	spec.Secrets = step.Secrets
	return spec
}
