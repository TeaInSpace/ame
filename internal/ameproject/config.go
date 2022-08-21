package ameproject

import (
	"errors"
	"os"
	"path"

	"gopkg.in/yaml.v3"
	"teainspace.com/ame/api/v1alpha1"
)

const AmeProjectFileName = "ame.yaml"

type TaskSpecs map[string]*v1alpha1.TaskSpec

type ProjectFileCfg struct {
	DefaultTask string    `yaml:"defaultTask,omitempty"`
	ProjectName string    `yaml:"projectname"`
	Specs       TaskSpecs `yaml:"tasks"`
}

type ProjectFileBuilder struct {
	fileCfg *ProjectFileCfg
}

func NewProjectFileBuilder() ProjectFileBuilder {
	return ProjectFileBuilder{
		fileCfg: &ProjectFileCfg{
			Specs: make(TaskSpecs),
		},
	}
}

func BuilderFromProjectFile(cfg *ProjectFileCfg) (ProjectFileBuilder, error) {
	if cfg == nil {
		return ProjectFileBuilder{}, errors.New("from BuilderFromProjectFile, received a nil ProjectFileCfg pointer")
	}

	return ProjectFileBuilder{
		fileCfg: cfg,
	}, nil
}

func (b ProjectFileBuilder) SetProjectName(name string) ProjectFileBuilder {
	b.fileCfg.ProjectName = name
	return b
}

func (b ProjectFileBuilder) SetDefaultTask(name string) ProjectFileBuilder {
	b.fileCfg.DefaultTask = name
	return b
}

func (b ProjectFileBuilder) AddTaskSpecs(specs TaskSpecs) ProjectFileBuilder {
	for name, spec := range specs {
		b.fileCfg.Specs[name] = spec
	}

	return b
}

func (b ProjectFileBuilder) Build() *ProjectFileCfg {
	return b.fileCfg
}

func WriteProjectFile(dir string, cfg *ProjectFileCfg) error {
	data, err := yaml.Marshal(cfg)
	if err != nil {
		return err
	}

	// TODO: check that these permissions are appropridate.
	return os.WriteFile(path.Join(dir, AmeProjectFileName), data, 0o777)
}

// TODO: support all yaml file extensions
func ReadProjectFile(dir string) (*ProjectFileCfg, error) {
	data, err := os.ReadFile(path.Join(dir, AmeProjectFileName))
	if err != nil {
		return nil, err
	}

	out := &ProjectFileCfg{}
	err = yaml.Unmarshal(data, out)
	if err != nil {
		return nil, err
	}

	for key := range out.Specs {
		out.Specs[key].ProjectId = out.ProjectName
	}

	return &ProjectFileCfg{
		ProjectName: out.ProjectName,
		Specs:       out.Specs,
	}, nil
}
