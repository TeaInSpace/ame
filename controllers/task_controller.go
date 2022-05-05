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

package controllers

import (
	"context"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/handler"
	"sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/source"

	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
)

// TaskReconciler reconciles a Task object
type TaskReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

//+kubebuilder:rbac:groups=ame.teainspace.com,resources=tasks,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=ame.teainspace.com,resources=tasks/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=ame.teainspace.com,resources=tasks/finalizers,verbs=update
//+kubebuilder:rbac:groups=argoproj.io,resources=Workflows,verbs=get;list;watch;create;update;patch;delete

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
// TODO(user): Modify the Reconcile function to compare the state specified by
// the Task object against the actual cluster state, and then
// perform operations to make the cluster state reflect the state specified by
// the user.
//
// For more details, check Reconcile and its Result here:
// - https://pkg.go.dev/sigs.k8s.io/controller-runtime@v0.11.0/pkg/reconcile
func (r *TaskReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	// TODO: verify that workflows are correctly configured, even if they already exist.
	// TODO: ensure that standard fields such as status are updated appropriately.
	log := log.FromContext(ctx)
	var task amev1alpha1.Task

	err := r.Get(ctx, req.NamespacedName, &task)
	if err != nil {
		log.Error(err, "Unable to get a task")
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}

	var wf argo.Workflow
	err = getArgoWorkflow(ctx, r.Client, task, &wf)
	if err == nil {
		log.Info("Workflow already present for task")
		return ctrl.Result{}, nil
	}

	ownerRef, err := amev1alpha1.TaskOwnerRef(r.Scheme, task)
	if err != nil {
		log.Error(err, "Unable to generate OwnerReference for Task")
		return ctrl.Result{}, err
	}

	wf = genArgoWorkflow(task, ownerRef)
	err = r.Create(ctx, &wf)
	// TODO: Should error messages have punctuation?
	if err != nil {
		log.Error(err, "Unable able to create argo workflow")
		return ctrl.Result{}, err
	}

	log.Info("Workflow created for task " + task.GetName())

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *TaskReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&amev1alpha1.Task{}).
		Watches(
			&source.Kind{
				Type: &argo.Workflow{},
			},
			&handler.EnqueueRequestForOwner{
				OwnerType:    &amev1alpha1.Task{},
				IsController: false,
			},
		).
		Complete(r)
}
