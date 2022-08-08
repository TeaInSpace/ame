package dirtools

import (
	"bytes"
	"fmt"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"strings"
	"testing"

	"teainspace.com/ame/server/storage"
)

var testingFiles = []storage.ProjectFile{
	{
		Path: "somefile",
		Data: []byte("somedata"),
	},
	{
		Path: ".hiddenfile",
		Data: []byte("hiddendata"),
	},
	{
		Path: "somedir/anotherfile",
		Data: []byte("moredata"),
	},
	{
		Path: "somedir/anotheranotherfile.txt",
		Data: []byte("moremoredata"),
	},
	{
		Path: "somedir/anotherdir/morefile.json",
		Data: []byte("{this is json}"),
	},
}

// diffFiles validates that the files in actualFiles matches the files in expectedFiles.
func diffFiles(expectedFiles []storage.ProjectFile, actualFiles []storage.ProjectFile) []string {
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

func removeParentDir(path string, prefix string) string {
	return strings.Replace(path, prefix+"/", "", 1)
}

// testDirMatchesFiles checks if the supplied directory contains the files in the
// testingFiles list and no other files. It ignores empty directories.
func testDirMatchesFiles(dir string) error {
	filesCreated := []storage.ProjectFile{}
	filepath.WalkDir(dir, func(entryPath string, d fs.DirEntry, err error) error {
		if d.IsDir() {
			return nil
		}

		data, err := os.ReadFile(entryPath)
		if err != nil {
			return err
		}

		// The relative filepath is stored as this makes it possible to compare
		// with the list of testFiles.
		filesCreated = append(filesCreated, storage.ProjectFile{
			Path: removeParentDir(entryPath, dir),
			Data: data,
		})

		return nil
	})

	diffs := diffFiles(testingFiles, filesCreated)
	if len(diffs) > 0 {
		return fmt.Errorf("%+v\n\n expected: %+v\n\n%v", filesCreated, testingFiles, diffs)
	}

	return nil
}

func TestPopulateDir(t *testing.T) {
	tempDir, err := os.MkdirTemp("", "mydir")
	if err != nil {
		t.Error(err)
	}

	err = PopulateDir(tempDir, testingFiles)
	if err != nil {
		t.Error(err)
	}

	err = testDirMatchesFiles(tempDir)
	if err != nil {
		t.Errorf("PopulateDir created: %s", err)
	}
}

func TestMkDirTempAndApplyReturnsCorrectError(t *testing.T) {
	tempInput := "myproject"
	tempDir, err := MkAndPopulateDirTemp(tempInput, testingFiles)
	if err != nil {
		t.Error(err)
	}

	err = testDirMatchesFiles(tempDir)
	if err != nil {
		t.Errorf("MkAndPopulateDirTemp created: %s", err)
	}

	if !strings.HasPrefix(path.Base(tempDir), tempInput) {
		t.Errorf("The name of the temporary directory: %s is missing the prefix %s", tempDir, tempInput)
	}
}
