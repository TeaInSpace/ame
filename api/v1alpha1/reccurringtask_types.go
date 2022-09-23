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

// ReccurringTaskSpec defines the desired state of ReccurringTask
type ReccurringTaskSpec struct {
	// INSERT ADDITIONAL SPEC FIELDS - desired state of cluster
	// Important: Run "make" to regenerate code after modifying this file

	// TaskRef references a Task by name.
	TaskSpec TaskSpec `json:"taskSpec"`

	// Cron schedule used for scheduling Task execution.
	Schedule string `json:"schedule"`
}

// ReccurringTaskStatus defines the observed state of ReccurringTask
type ReccurringTaskStatus struct {
	// INSERT ADDITIONAL STATUS FIELD - define observed state of cluster
	// Important: Run "make" to regenerate code after modifying this file
}

//+genclient
//+kubebuilder:object:root=true
//+kubebuilder:subresource:status

// ReccurringTask is the Schema for the reccurringtasks API
type ReccurringTask struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   ReccurringTaskSpec   `json:"spec,omitempty"`
	Status ReccurringTaskStatus `json:"status,omitempty"`
}

//+kubebuilder:object:root=true

// ReccurringTaskList contains a list of ReccurringTask
type ReccurringTaskList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []ReccurringTask `json:"items"`
}

func init() {
	SchemeBuilder.Register(&ReccurringTask{}, &ReccurringTaskList{})
}

func NewRecurringTask(namePrefix string, taskSpec TaskSpec, schedule string) *ReccurringTask {
	return &ReccurringTask{
		ObjectMeta: metav1.ObjectMeta{
			GenerateName: namePrefix,
		},
		Spec: ReccurringTaskSpec{
			TaskSpec: taskSpec,
			Schedule: schedule,
		},
		Status: ReccurringTaskStatus{},
	}
}

// RecurringTaskOwnerRef returns an OwnerReference referencing the given Task.
func ReccuringTaskOwnerRef(scheme *runtime.Scheme, recTask *ReccurringTask) (metav1.OwnerReference, error) {
	gvks, err := GenGvks(scheme, recTask)
	if len(gvks) == 0 {
		return metav1.OwnerReference{}, errors.Errorf("Could not find a GroupVersionKind for task: %s", recTask.GetName())
	}
	// TODO: Add support for multiple GroupVersionKinds.

	return GenOwnRef(recTask.ObjectMeta, gvks), err
}
