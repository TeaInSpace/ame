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
	"fmt"
	"reflect"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	argoClients "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/handler"
	"sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/source"

	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/workflows"
)

// ReccurringTaskReconciler reconciles a ReccurringTask object
type ReccurringTaskReconciler struct {
	client.Client
	Scheme  *runtime.Scheme
	CronWfs argoClients.CronWorkflowInterface
}

//+kubebuilder:rbac:groups=ame.teainspace.com,resources=reccurringtasks,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=ame.teainspace.com,resources=reccurringtasks/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=ame.teainspace.com,resources=reccurringtasks/finalizers,verbs=update
//+kubebuilder:rbac:groups=argoproj.io,resources=cronworkflows,verbs=get;list;watch;create;update;patch;delete

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
// TODO(user): Modify the Reconcile function to compare the state specified by
// the ReccurringTask object against the actual cluster state, and then
// perform operations to make the cluster state reflect the state specified by
// the user.
//
// For more details, check Reconcile and its Result here:
// - https://pkg.go.dev/sigs.k8s.io/controller-runtime@v0.11.0/pkg/reconcile
func (r *ReccurringTaskReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := log.FromContext(ctx)

	var rTask amev1alpha1.ReccurringTask
	err := r.Get(ctx, req.NamespacedName, &rTask)
	if err != nil {
		log.Error(err, "Unable to get recurring task")
		return ctrl.Result{}, err
	}

	cronWf, err := workflows.CronWfForRecTask(ctx, r.CronWfs, rTask.GetName())
	if err == nil {
		err := r.reconcileCronWfSpec(ctx, &rTask, cronWf)
		if err != nil {
			log.Error(err, "unable to validatd CronWorkflow")
			return ctrl.Result{}, err
		}

		return ctrl.Result{}, nil
	}

	cwf, err := workflows.GenCronWf(&rTask, r.Scheme)
	if err != nil {
		log.Error(err, "failed to generate CronWorkflow object")
	}

	_, err = r.CronWfs.Create(ctx, cwf, v1.CreateOptions{})
	if err != nil {
		fmt.Println(err)
		log.Error(err, "failed to create CronWorflow")
		return ctrl.Result{}, err
	}

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *ReccurringTaskReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&amev1alpha1.ReccurringTask{}).
		Watches(&source.Kind{
			Type: &argo.CronWorkflow{},
		},
			&handler.EnqueueRequestForOwner{
				OwnerType:    &amev1alpha1.ReccurringTask{},
				IsController: false,
			},
		).Complete(r)
}

// reconcileCronWfSpec ensures that the in cluster spec for a CronWorkflow is aligned with recTask.
func (r *ReccurringTaskReconciler) reconcileCronWfSpec(ctx context.Context, recTask *amev1alpha1.ReccurringTask, cronWf *argo.CronWorkflow) error {
	correctCronWf, err := workflows.GenCronWf(recTask, r.Scheme)
	if err != nil {
		return err
	}

	if reflect.DeepEqual(correctCronWf.Spec, cronWf.Spec) {
		return nil
	}

	cronWf.Spec = correctCronWf.Spec
	err = r.Update(ctx, cronWf)
	if err != nil {
		return err
	}

	return nil
}
