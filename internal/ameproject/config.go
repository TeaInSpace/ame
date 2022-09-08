package ameproject

import (
	"errors"
	"os"
	"path"

	"gopkg.in/yaml.v3"
	"teainspace.com/ame/api/v1alpha1"
)

const AmeProjectFileName = "ame.yaml"

// A TaskSpecName is how we identify individual TaskSpecs created
// by the user in the AME project file. This will be how a user
// identifies a task specification.
type TaskSpecName string

// A TaskSpecs instance maps TaskSpecNames to TaskSpecs.
type TaskSpecs map[TaskSpecName]*v1alpha1.TaskSpec

// A ProjectFileCfg encapsulats all of the configuration in an
// AME project file.
type ProjectFileCfg struct {
	DefaultTask string    `yaml:"defaultTask,omitempty"`
	ProjectName string    `yaml:"projectname"`
	Specs       TaskSpecs `yaml:"tasks"`
}

// A ProjectFileBuilder allows for procedural creation of a ProjectFileCfg,
// by having individual methods for each aspect of the AME project file configuration.
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

// WriteToProjectFile saves a ProjectFileCfg to a directory, using AmeProjectFileName as the file name.
// If there is an existing AME project file, the two configurations are merged, where the values in cfg
// take precedence over existing values.
// An error is returned if something goes wrong.
func WriteToProjectFile(dir string, cfg *ProjectFileCfg) error {
	ok, err := ValidProjectCfgExists(dir)
	if err != nil {
		return err
	}

	if ok {
		existingCfg, err := ReadProjectFile(dir)
		if err != nil {
			return err
		}

		for key := range cfg.Specs {
			existingCfg.Specs[key] = cfg.Specs[key]
		}

		cfg.Specs = existingCfg.Specs
		cfg.ProjectName = existingCfg.ProjectName
	}

	data, err := yaml.Marshal(cfg)
	if err != nil {
		return err
	}

	// TODO: check that these permissions are appropridate.
	return os.WriteFile(path.Join(dir, AmeProjectFileName), data, 0o777)
}

// TODO: support all yaml file extensions

// ReadProjectFile attempts to generated a ProjectFileCfg from a file in dir with AmeProjectFileName as the name.
// The structure of the file is validated, but the values are not.
// A pointer to the generated configuration is returned if no errors are encountered.
// If an error is encountered a nil pointer is returned a long with the error.
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

// ValidProjectCfgExists determines is a valid AME project file exists in dir.
// A bool is returned indicating if that is the case.
// If an error is encountered it is assumed that no valid file was found.
func ValidProjectCfgExists(dir string) (bool, error) {
	cfg, err := ReadProjectFile(dir)
	if os.IsNotExist(err) {
		return false, nil
	}

	if err != nil {
		return false, err
	}

	if cfg.ProjectName == "" {
		return false, nil
	}

	return true, nil
}
