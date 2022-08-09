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

// correctWorkflowSpec returns a corrected Workflow specification if the given Workflow specification
// is not correct for the given Task. A boolean is returned aswell indicating is the Workflow speci-
// fication  should be updated, true if the specification should be updated and false if not.
func correctWorkflowSpec(taskSpec amev1alpha1.TaskSpec, wf argo.Workflow) (argo.WorkflowSpec, bool) {
	correctWorkflowSpec := genWorkflowSpec(taskSpec)

	if len(correctWorkflowSpec.Arguments.Parameters) != len(wf.Spec.Arguments.Parameters) {
		return correctWorkflowSpec, true
	}

	for i, correctParam := range correctWorkflowSpec.Arguments.Parameters {
		if correctParam.Name != wf.Spec.Arguments.Parameters[i].Name || correctParam.Value != wf.Spec.Arguments.Parameters[i].Value {
			return correctWorkflowSpec, true
		}
	}

	return argo.WorkflowSpec{}, false
}

// genArgoWorkflow generates a Workflow object for the given Task.
func genArgoWorkflow(task amev1alpha1.Task, ownerRefs ...v1.OwnerReference) argo.Workflow {
	return argo.Workflow{
		ObjectMeta: v1.ObjectMeta{
			GenerateName:    workflowName(task.Name),
			Namespace:       task.Namespace,
			OwnerReferences: ownerRefs,
		},
		Spec: genWorkflowSpec(task.Spec),
	}
}

// genWorkflowSpec generates a Workflow specficiation from a Task specification.
func genWorkflowSpec(spec amev1alpha1.TaskSpec) argo.WorkflowSpec {
	return argo.WorkflowSpec{
		Arguments: argo.Arguments{
			Parameters: genParameters(spec),
		},
	}
}

// genParameters generates Workflow parameters from a Task specification.
func genParameters(spec amev1alpha1.TaskSpec) []argo.Parameter {
	return []argo.Parameter{
		{
			Name:  "project-id",
			Value: argo.AnyStringPtr(spec.ProjectId),
		},
		{
			Name:  "run-command",
			Value: argo.AnyStringPtr(spec.RunCommand),
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

func ExtractRunCommand(wf *argo.Workflow) string {
	return wf.Spec.Arguments.Parameters[1].Value.String()
}

func ExtractProjectID(wf *argo.Workflow) string {
	return wf.Spec.Arguments.Parameters[0].Value.String()
}
