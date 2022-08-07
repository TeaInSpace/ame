package dirtools

import (
	"bytes"
	"errors"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"strings"
	"testing"

	"teainspace.com/ame/server/storage"
)

var files = []storage.ProjectFile{
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

func TestPopulateDir(t *testing.T) {
	tempDir, err := os.MkdirTemp("", "mydir")
	if err != nil {
		t.Error(err)
	}

	err = ApplyFilesToDir(files)(tempDir)
	if err != nil {
		t.Error(err)
	}

	filesCreated := []storage.ProjectFile{}
	filepath.WalkDir(tempDir, func(entryPath string, d fs.DirEntry, err error) error {
		if d.IsDir() {
			return nil
		}

		data, err := os.ReadFile(entryPath)
		if err != nil {
			t.Error(err)
		}

		filesCreated = append(filesCreated, storage.ProjectFile{
			Path: strings.Replace(entryPath, tempDir+"/", "", 1),
			Data: data,
		})

		return nil
	})

	for _, f := range files {
		foundMatch := false
		for _, fc := range filesCreated {
			if f.Path == fc.Path {
				if bytes.Compare(f.Data, fc.Data) != 0 {
					t.Errorf("Found file %s with mismatching data between original %s and created file %s", f.Path, string(f.Data), string(fc.Data))
				}

				foundMatch = true
				continue
			}
		}

		if !foundMatch {
			t.Errorf("Did not match created for %s", f.Path)
		}
	}

	if len(files) != len(filesCreated) {
		t.Errorf("Number of created files %d does not match the expected amount %d", len(filesCreated), len(files))
	}
}

func TestMkDirTempAndApplyReturnsCorrectError(t *testing.T) {
	var dirInApply string
	testErr := errors.New("test error")
	apply := func(dir string) error {
		dirInApply = dirInApply
		return testErr
	}

	tempInput := "myproject"
	_, err := MkDirTempAndApply(tempInput, apply)

	if err != testErr {
		t.Errorf("Expected to receive the test error %s but got %s instead", testErr, err)
	}
}

func TestMkdirDirTempAndApplyCreatesCorrectDirectoryName(t *testing.T) {
	var dirInApply string
	apply := func(dir string) error {
		dirInApply = dirInApply
		return nil
	}

	tempInput := "myproject"
	tempDir, err := MkDirTempAndApply(tempInput, apply)
	if err != nil {
		t.Error(err)
	}

	if !strings.HasPrefix(path.Base(tempDir), tempInput) {
		t.Errorf("The name of the temporary directory: %s is missing the prefix %s", tempDir, tempInput)
	}
}
