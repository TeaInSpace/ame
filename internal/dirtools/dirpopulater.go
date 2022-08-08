package dirtools

import (
	"bytes"
	"fmt"
	"os"
	"path"
	"strings"

	"teainspace.com/ame/server/storage"
)

// MkAndPopulateDirTemp creates a directory in the default location for temporary files,
// generating a random name with the supplied name parameter as a prefix. The
// directory is then populated with the supplied files. See os.MkDirTemp for details on
// how temporary directories are created and named.
// The returned string contains the path to the temporay directory. If the operation
// fails at any point an error will be returned indidcating what happened.
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
//
// # Example:
//
// if dir=/path/to/mydir and files contains a path relativedir/myfile,
// this will result in placing the file at /path/tov1alphamydir/relativedir/myfile.
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

func RemoveParentDir(path string, prefix string) string {
	return strings.Replace(path, prefix+"/", "", 1)
}

// DiffFiles validates that the files in actualFiles matches the files in expectedFiles.
func DiffFiles(expectedFiles []storage.ProjectFile, actualFiles []storage.ProjectFile) []string {
	diffs := []string{}
	diffFile := func(fExpected storage.ProjectFile, fActual storage.ProjectFile) (bool, string) {
		if fExpected.Path == fActual.Path {
			if !bytes.Equal(fExpected.Data, fActual.Data) {
				return true, fmt.Sprintf("file %s has mismatching data expected: %s actual: %s", fExpected.Path, string(fExpected.Data), string(fActual.Data))
			}

			return true, ""
		}

		return false, ""
	}

	for _, fExpected := range expectedFiles {
		foundMatch := false
		for _, fActual := range actualFiles {
			pathMatch, diff := diffFile(fExpected, fActual)
			if diff != "" {
				diffs = append(diffs, diff)
			}

			// Here we make sure that if we have previously found a match
			// foundMatch is not overwritten.
			// Normally we would exit early when a match is found, but we
			// want to ensure that all errors are caught and therefore we
			// do no exit early.
			foundMatch = foundMatch || pathMatch && diff == ""
		}

		if !foundMatch {
			diffs = append(diffs, fmt.Sprintf("Missing file: %s", fExpected.Path))
		}
	}

	if len(expectedFiles) != len(actualFiles) {
		diffs = append(diffs, fmt.Sprintf("Number of actual files %d does not match the expected amount %d", len(actualFiles), len(expectedFiles)))
	}

	return diffs
}
