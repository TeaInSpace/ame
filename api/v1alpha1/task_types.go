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
	"github.com/pkg/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	runtime "k8s.io/apimachinery/pkg/runtime"
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
}

// TaskStatus defines the observed state of Task
type TaskStatus struct {
	// INSERT ADDITIONAL STATUS FIELD - define observed state of cluster
	// Important: Run "make" to regenerate code after modifying this file
}

// +genclient
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
func TaskOwnerRef(scheme *runtime.Scheme, task Task) (metav1.OwnerReference, error) {
	gvks, _, err := scheme.ObjectKinds(&task)
	if err != nil {
		return metav1.OwnerReference{}, err
	}

	if len(gvks) == 0 {
		return metav1.OwnerReference{}, errors.Errorf("Could not find a GroupVersionKind for task: %s", task.GetName())
	}
	// TODO: Add support for multiple GroupVersionKinds.

	return metav1.OwnerReference{
		APIVersion: gvks[0].GroupVersion().String(), // TODO: is there a better method for getting the APIVersion string?
		Kind:       gvks[0].Kind,
		Name:       task.GetName(),
		UID:        task.GetUID(),
	}, err
}
