package ameproject

import (
	"testing"

	"teainspace.com/ame/api/v1alpha1"
)

func TestCanBuildProjectFileCfg(t *testing.T) {
	specs := TaskSpecs{
		"updatedata": &v1alpha1.TaskSpec{
			RunCommand: "python update_data.py",
			ProjectId:  "myproject",
		},
		"training": &v1alpha1.TaskSpec{
			RunCommand: "python train.py",
			ProjectId:  "myproject",
		},
	}

	projectName := "myproject"
	defaultTask := "training"

	builder := NewProjectFileBuilder()

	builder = builder.SetProjectName(projectName)

	if builder.fileCfg.ProjectName != projectName {
		t.Errorf("expected builder.Filecfg.ProjectName=%s, but got %s instead", projectName, builder.fileCfg.ProjectName)
	}

	builder = builder.SetDefaultTask(defaultTask)

	if builder.fileCfg.DefaultTask != defaultTask {
		t.Errorf("expected builder.fileCfg.DefaultTask=%s, but got %s instead", defaultTask, builder.fileCfg.ProjectName)
	}

	builder = builder.AddTaskSpecs(specs)
	if len(specs) != len(builder.fileCfg.Specs) {
		t.Errorf("expected builder.fileCfg.Specs to have length %v, but got length %v instead", len(specs), len(builder.fileCfg.Specs))
	}
}
