package controllers

import (
	"testing"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
)

func genWf(projectID string, taskId string, runCmd string) *argo.Workflow {
	parameters := []argo.Parameter{
		{
			Name:  "task-id",
			Value: argo.AnyStringPtr(taskId),
		},
		{
			Name:  "project-id",
			Value: argo.AnyStringPtr(projectID),
		},
		{
			Name:  "run-command",
			Value: argo.AnyStringPtr(runCmd),
		},
	}

	return &argo.Workflow{
		Spec: argo.WorkflowSpec{
			Arguments: argo.Arguments{
				Parameters: parameters,
			},
		},
	}
}

func TestExtractRunCommnd(t *testing.T) {
	runCmd := "myruncmd"
	wf := genWf("", "", runCmd)

	extractedRunCmd := ExtractRunCommand(wf)
	if extractedRunCmd != runCmd {
		t.Errorf("expected ExtractRunCommand(wf)=%s, got %s instead", runCmd, extractedRunCmd)
	}
}

func TestExtractProjectID(t *testing.T) {
	projectID := "myproject"
	wf := genWf(projectID, "", "")

	extractedProjectID := ExtractProjectID(wf)
	if extractedProjectID != projectID {
		t.Errorf("expected ExtractProjectID(wf)=%s, got %s instead", projectID, extractedProjectID)
	}
}
