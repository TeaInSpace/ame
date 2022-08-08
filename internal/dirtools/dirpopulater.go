package dirtools

import (
	"os"
	"path"

	"teainspace.com/ame/server/storage"
)

// MkAndPopulateDirTemp creates a directory in the default location for temporary files.
// generating a random name with the supplied name parameter as a prefix. The
// directory is then populated with the supplied files.
func MkAndPopulateDirTemp(name string, files []storage.ProjectFile) (string, error) {
	// The dir to Mkdirtemp is left empty since we want to use the default location
	// for temporary files.
	path, err := os.MkdirTemp("", name)
	if err != nil {
		return "", err
	}

	err = PopulateDir(path, files)
	if err != nil {
		return "", err
	}

	return path, nil
}

// PoplateDir populates a directory with the supplied files, under the
// expecation that the supplied files have relative paths.
func PopulateDir(dir string, files []storage.ProjectFile) error {
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
