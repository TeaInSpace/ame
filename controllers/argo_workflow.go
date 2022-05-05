package controllers

import (
	"context"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
)

type workflowNotFoundError struct {
	task amev1alpha1.Task
}

func (e workflowNotFoundError) Error() string {
	return "Workflow not found for " + e.task.GetName()
}

func newWorkflowNotFoundError(task amev1alpha1.Task) workflowNotFoundError {
	return workflowNotFoundError{task}
}

func workflowName(taskName string) string {
	return taskName + "-wf"
}

// genArgoWorkflow generates a Workflow object for the given task.
func genArgoWorkflow(task amev1alpha1.Task, ownerRefs ...v1.OwnerReference) argo.Workflow {
	return argo.Workflow{
		ObjectMeta: v1.ObjectMeta{
			GenerateName:    workflowName(task.Name),
			Namespace:       task.Namespace,
			OwnerReferences: ownerRefs,
		},
	}
}

// getArgoWorkflow retrieves the workflow owned by the task, if such a workflow exists the out object will be populated
// with that workflow.
func getArgoWorkflow(ctx context.Context, k8sClient client.Client, task amev1alpha1.Task, out *argo.Workflow) error {
	// TODO: Find an alternative method of gettting the workflow for a task, without fetching the entire list and filtering it.
	// TODO: How should we handle the possibility of multiple workflows owned by a single task?
	workflows := argo.WorkflowList{}
	err := k8sClient.List(ctx, &workflows, &client.ListOptions{Namespace: task.GetNamespace()})
	if err != nil {
		return err
	}

	for _, wf := range workflows.Items {
		for _, or := range wf.GetOwnerReferences() {
			if or.UID == task.GetUID() {
				wf.DeepCopyInto(out)
				return nil
			}
		}
	}

	return newWorkflowNotFoundError(task)
}
