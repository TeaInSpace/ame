package controllers

import (
	"context"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	apiv1 "k8s.io/api/core/v1"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/workflows"
)

type workflowNotFoundError struct {
	task amev1alpha1.Task
}

func (e workflowNotFoundError) Error() string {
	return "Workflow not found for " + e.task.GetName()
}

func NewWorkflowNotFoundError(task amev1alpha1.Task) workflowNotFoundError {
	return workflowNotFoundError{task}
}

func workflowName(taskName string) string {
	return taskName + "-wf"
}

// TODO: refactor to use errors properly

// correctWorkflowSpec returns a corrected Workflow specification if the given Workflow specification
// is not correct for the given Task. A boolean is returned aswell indicating is the Workflow speci-
// fication  should be updated, true if the specification should be updated and false if not.
func correctWorkflowSpec(task amev1alpha1.Task, wf argo.Workflow) (argo.WorkflowSpec, bool) {
	correctWorkflowSpec, err := genWorkflowSpec(task)
	if err != nil {
		return argo.WorkflowSpec{}, false
	}

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
func genArgoWorkflow(task amev1alpha1.Task, ownerRefs ...v1.OwnerReference) (argo.Workflow, error) {
	spec, err := genWorkflowSpec(task)
	if err != nil {
		return argo.Workflow{}, err
	}

	return argo.Workflow{
		ObjectMeta: v1.ObjectMeta{
			GenerateName:    workflowName(task.Name),
			Namespace:       task.Namespace,
			OwnerReferences: ownerRefs,
		},
		Spec: spec,
	}, nil
}

// genWorkflowSpec generates a Workflow specficiation from a Task specification.
func genWorkflowSpec(task amev1alpha1.Task) (argo.WorkflowSpec, error) {
	if len(task.Spec.Pipeline) > 0 {
		spec, err := workflows.GenPipelineWf(&task)
		if err != nil {
			return argo.WorkflowSpec{}, err
		}

		return *spec, nil
	}

	wfTemplates := []argo.Template{
		{
			Name: "main",
			Inputs: argo.Inputs{
				Parameters: []argo.Parameter{
					{
						Name:  "memory-limit",
						Value: argo.AnyStringPtr("3Gi"),
					},
				},
			},

			Script: &argo.ScriptTemplate{
				Source: `

          export TASK_DIRECTORY=ameprojectstorage/{{workflow.parameters.project-id}}

          s3cmd --no-ssl --region us-east-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://$TASK_DIRECTORY ./

          cd "./{{workflow.parameters.project-id}}" 

          set -e # It is important that the workflow exits with an error code if execute or save_artifacts fails, so AME can take action based on that information.

          execute "{{workflow.parameters.run-command}}" 
          
          save_artifacts "ameprojectstorage/{{workflow.parameters.task-id}}/artifacts/"

          echo "0" >> exit.status
					`,
				Container: apiv1.Container{
					Name:  "ame-executor",
					Image: "ame-executor:local",
					Command: []string{
						"bash",
					},
					Env: append([]apiv1.EnvVar{
						{
							Name:  "AWS_ACCESS_KEY_ID",
							Value: "minio",
						},
						{
							Name:  "AWS_SECRET_ACCESS_KEY",
							Value: "minio123",
						},
						{
							Name:  "MINIO_URL",
							Value: "http://ame-minio.ame-system.svc.cluster.local:9000",
						},
						{
							Name:  "PIPENV_YES",
							Value: "1",
						},
					}, workflows.TaskEnvToContainerEnv(task.Spec)...),
				},
			},

			PodSpecPatch: `{"containers":[{"name":"main", "resources":{"limits":{
        "memory": "{{inputs.parameters.memory-limit}}"   }}}]}`,
		},
	}

	wfSpec := *workflows.GenWfSpec(task.GetName(), []apiv1.PersistentVolumeClaim{}, wfTemplates)
	wfSpec.Arguments.Parameters = genParameters(task)
	return wfSpec, nil
}

// genParameters generates Workflow parameters from a Task specification.
func genParameters(task amev1alpha1.Task) []argo.Parameter {
	return []argo.Parameter{
		{
			Name:  "task-id",
			Value: argo.AnyStringPtr(task.GetName()),
		},
		{
			Name:  "project-id",
			Value: argo.AnyStringPtr(task.Spec.ProjectId),
		},
		{
			Name:  "run-command",
			Value: argo.AnyStringPtr(task.Spec.RunCommand),
		},
	}
}

// GetArgoWorkflow retrieves the workflow owned by the task, if such a workflow exists the out object will be populated
// with that workflow.
func GetArgoWorkflow(ctx context.Context, k8sClient client.Client, task amev1alpha1.Task, out *argo.Workflow) error {
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

	return NewWorkflowNotFoundError(task)
}

func ExtractRunCommand(wf *argo.Workflow) string {
	return wf.Spec.Arguments.Parameters[2].Value.String()
}

func ExtractProjectID(wf *argo.Workflow) string {
	return wf.Spec.Arguments.Parameters[1].Value.String()
}
