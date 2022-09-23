package workflows

import (
	"context"
	"fmt"
	"time"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	argoClients "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
)

const recTaskLabelKey = "ame-recurring-task"

func GenCronWf(recTask *amev1alpha1.ReccurringTask, scheme *runtime.Scheme) (*argo.CronWorkflow, error) {
	task := amev1alpha1.NewTaskFromSpec(&recTask.Spec.TaskSpec, recTask.GetName())
	// We have to set the name afterwards here, as NewTaskFromSpec sets the GenerateName field, which is not useful
	// in this case as we are never creating the Task within a Cluster, but rather using it to generate a WorkflowSpec.
	task.SetName(recTask.GetName())
	wfSpec, err := GenWorkflowSpec(*task)
	if err != nil {
		return nil, err
	}

	ownerRefs, err := amev1alpha1.ReccuringTaskOwnerRef(scheme, recTask)
	if err != nil {
		return nil, err
	}

	return &argo.CronWorkflow{
		ObjectMeta: GenObjMeta(recTask.GetName(), recTask.GetNamespace(), GenRecTaskLabels(recTask), []metav1.OwnerReference{ownerRefs}),
		Spec: argo.CronWorkflowSpec{
			Schedule:     recTask.Spec.Schedule,
			WorkflowSpec: wfSpec,
		},
	}, nil
}

func GenObjMeta(namePrefix string, ns string, labels map[string]string, ownerRefs []metav1.OwnerReference) metav1.ObjectMeta {
	return metav1.ObjectMeta{
		GenerateName:    namePrefix,
		Namespace:       ns,
		Labels:          labels,
		OwnerReferences: ownerRefs,
	}
}

func GenRecTaskLabels(recTask *amev1alpha1.ReccurringTask) map[string]string {
	return map[string]string{
		recTaskLabelKey: recTask.GetName(),
	}
}

func WaitForCronWfForRecTask(ctx context.Context, cronWfs argoClients.CronWorkflowInterface, name string, timeout time.Duration) (*argo.CronWorkflow, error) {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	deadline, _ := ctx.Deadline()

	for {
		cronWf, err := CronWfForRecTask(ctx, cronWfs, name)
		if err != nil && time.Now().Before(deadline) {
			continue
		}

		if err != nil {
			return nil, err
		}

		return cronWf, nil
	}
}

func CronWfForRecTask(ctx context.Context, cronWfs argoClients.CronWorkflowInterface, name string) (*argo.CronWorkflow, error) {
	selector, err := labels.Parse(fmt.Sprintf("%s=%s", recTaskLabelKey, name))
	if err != nil {
		return nil, err
	}

	cwfs, err := cronWfs.List(ctx, metav1.ListOptions{LabelSelector: selector.String()})
	if err != nil {
		return nil, err
	}

	if len(cwfs.Items) == 0 {
		return nil, fmt.Errorf("found no CronWorkflows for the ReccurringTask: %s", name)
	}

	if len(cwfs.Items) > 1 {
		return nil, fmt.Errorf("expected recurring task controller to only create 1 CronWorkflow but got %d instead", len(cwfs.Items))
	}

	return &cwfs.Items[0], nil
}
