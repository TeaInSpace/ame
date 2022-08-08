package dirtools

import (
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
			Path: RemoveParentDir(entryPath, dir),
			Data: data,
		})

		return nil
	})

	diffs := DiffFiles(testingFiles, filesCreated)
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
