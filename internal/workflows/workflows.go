// Package workflows implements utility functions and types for working with Argo Workflows.
//
// AME uses Argo Workflows as the orchestrator for executing Tasks, by converting Tasks to Workflows.
package workflows

import (
	"context"
	"fmt"
	"time"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	argoClients "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
	apiv1 "k8s.io/api/core/v1"
	v1resources "k8s.io/apimachinery/pkg/api/resource"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/labels"
	"teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/internal/secrets"
)

const taskLabelKey = "ame-task"

// genWorkflowSpec generates a Workflow specficiation from a Task specification.
func GenWorkflowSpec(task v1alpha1.Task) (argo.WorkflowSpec, error) {
	if len(task.Spec.Pipeline) > 0 {
		spec, err := GenPipelineWf(&task)
		if err != nil {
			return argo.WorkflowSpec{}, err
		}

		return *spec, nil
	}

	wfTemplates := []argo.Template{genWfTemplate("main", &task, taskVolName(&task))}
	pvClaims, err := genPvcs(&task)
	if err != nil {
		return argo.WorkflowSpec{}, err
	}

	wfSpec := *GenWfSpec(task.GetName(), pvClaims, wfTemplates)
	return wfSpec, nil
}

// GenPipelineWf constructs a WorkflowSpec for a Task containing a pipeline.
// Note all other fields in the TaskSpec within the Task are ignored, as it is expected to only contain a pipeline.
// A pointer to the WorkflowSpec is returned, if any errors are encountered they are returned alog with a nil pointer.
func GenPipelineWf(t *v1alpha1.Task) (*argo.WorkflowSpec, error) {
	var wfSteps []argo.ParallelSteps
	for _, s := range t.Spec.Pipeline {
		tSpec := v1alpha1.WfSpecFromPipelineStep(t, s)
		wfTemplate := genWfTemplate(s.TaskName, v1alpha1.NewTaskFromSpec(tSpec, t.GetName()+s.TaskName), taskVolName(t))
		wfSteps = append(wfSteps, argo.ParallelSteps{
			Steps: []argo.WorkflowStep{
				{
					Inline: &wfTemplate,
					Name:   s.TaskName,
				},
			},
		})
	}

	pvcs, err := genPvcs(t)
	if err != nil {
		return nil, err
	}

	return GenWfSpec(t.GetName(), pvcs, []argo.Template{{
		Steps: wfSteps,
		Name:  "main",
	}}), nil
}

func genPvcs(t *v1alpha1.Task) ([]apiv1.PersistentVolumeClaim, error) {
	volStoreageReq, err := v1resources.ParseQuantity("5Gi")
	if err != nil {
		return nil, err
	}

	return []apiv1.PersistentVolumeClaim{
		{
			ObjectMeta: v1.ObjectMeta{
				Name: taskVolName(t),
			},
			Spec: apiv1.PersistentVolumeClaimSpec{
				AccessModes: []apiv1.PersistentVolumeAccessMode{
					apiv1.ReadWriteOnce,
				},
				Resources: apiv1.ResourceRequirements{
					Requests: apiv1.ResourceList{
						"storage": volStoreageReq,
					},
				},
			},
		},
	}, nil
}

// GenWfSpec constructs a WorkflowSpec.
// An ame-task label is included in the Pod metadata, using taskName as the value. This is intended to allow for easy identification of
// pods beloning to a Task.
// volClaimTemplates and wfTemplates are used in the WorkflowSpec without modification.
// The entrypoint is set to main, therefore wfTemplates must contain a template named main for the WorkflowSpec to be valid.
// A pointer to the WorkflowSpec is returned.
func GenWfSpec(taskName string, volClaimTemplates []apiv1.PersistentVolumeClaim, wfTemplates []argo.Template) *argo.WorkflowSpec {
	return &argo.WorkflowSpec{
		PodMetadata: &argo.Metadata{
			Labels: map[string]string{
				"ame-task": taskName,
			},
		},
		Templates:            wfTemplates,
		Entrypoint:           "main",
		VolumeClaimTemplates: volClaimTemplates,
	}
}

func taskVolName(t *v1alpha1.Task) string {
	return fmt.Sprintf("%s-volume", t.GetName())
}

func genWfTemplate(templateName string, t *v1alpha1.Task, volName string) argo.Template {
	return argo.Template{
		Name: templateName,

		Script: &argo.ScriptTemplate{
			Source: fmt.Sprintf(`

          export TASK_DIRECTORY=ameprojectstorage/%s

          s3cmd --no-ssl --region us-east-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://$TASK_DIRECTORY ./

          cd "./%s" 

          set -e # It is important that the workflow exits with an error code if execute or save_artifacts fails, so AME can take action based on that information.

          execute "%s" 
          
          save_artifacts "ameprojectstorage/%s/artifacts/"

          echo "0" >> exit.status
					`, t.Spec.ProjectId, t.Spec.ProjectId, t.Spec.RunCommand, t.GetName()),
			Container: apiv1.Container{
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
				}, TaskEnvToContainerEnv(t.Spec)...),
				VolumeMounts: []apiv1.VolumeMount{
					{
						Name:      volName,
						MountPath: "/project",
					},
				},
			},
		},

		PodSpecPatch: `{"containers":[{"name":"main", "resources":{"limits":{
        "memory": "3Gi"   }}}]}`,
	}
}

// TaskEnvToContainerEnv constructs an array of EnvVar from t's environment
// and secrets.
// The array of EnvVar is returned.
func TaskEnvToContainerEnv(t v1alpha1.TaskSpec) []apiv1.EnvVar {
	var v1env []apiv1.EnvVar
	for _, e := range t.Env {
		v1env = append(v1env, apiv1.EnvVar{
			Name:  e.Name,
			Value: e.Value,
		})
	}

	for _, s := range t.Secrets {
		v1env = append(v1env, apiv1.EnvVar{
			Name:      s.EnvKey,
			ValueFrom: secrets.SecretEnvVarSrc(s.Name),
		})
	}

	return v1env
}

func WaitForTaskWorkflow(ctx context.Context, workflows argoClients.WorkflowInterface, name string, timeout time.Duration) (*argo.Workflow, error) {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	deadline, _ := ctx.Deadline()

	for {
		wf, err := TaskWf(ctx, workflows, name)
		if err != nil && time.Now().Before(deadline) {
			continue
		}

		if err != nil {
			return nil, err
		}

		return wf, nil
	}
}

func TaskWf(ctx context.Context, workflows argoClients.WorkflowInterface, name string) (*argo.Workflow, error) {
	selector, err := labels.Parse(fmt.Sprintf("%s=%s", taskLabelKey, name))
	if err != nil {
		return nil, err
	}

	wfs, err := workflows.List(ctx, v1.ListOptions{LabelSelector: selector.String()})
	if err != nil {
		return nil, err
	}

	if len(wfs.Items) == 0 {
		return nil, fmt.Errorf("found no Workflows for the task %s", name)
	}

	if len(wfs.Items) > 1 {
		return nil, fmt.Errorf("expected recurring task controller to only create 1 Workflow but got %d instead", len(wfs.Items))
	}

	return &wfs.Items[0], nil
}
