package dirtools

import (
	"os"
	"path"

	"teainspace.com/ame/server/storage"
)

type DirApplyFunc func(string) error

// MkAndPopulateDirTemp creates a directory in the default location for temporary files.
// generating a random name with the supplied name parameter as a prefix. The
// directory is then populated with the supplied files.
func MkDirTempAndApply(name string, apply DirApplyFunc) (string, error) {
	// The dir to Mkdirtemp is left empty since we want to use the default location
	// for temporary files.
	path, err := os.MkdirTemp("", name)
	if err != nil {
		return "", err
	}

	err = apply(path)
	if err != nil {
		return "", err
	}

	return path, nil
}

// ApplyFilesToDir returns function which
// populates a directory with the supplied files, under the
// expecation that the supplied files have relative paths.
func ApplyFilesToDir(files []storage.ProjectFile) DirApplyFunc {
	return func(dir string) error {
		for _, f := range files {
			fPath := path.Join(dir, f.Path)
			if fDir := path.Dir(fPath); fDir != "" {
				err := os.MkdirAll(fDir, 0o755)
				if err != nil {
					return err
				}
			}

			err := os.WriteFile(fPath, f.Data, 0o644)
			if err != nil {
				return err
			}
		}

		return nil
	}
}
