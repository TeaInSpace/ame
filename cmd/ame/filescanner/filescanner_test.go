package filescanner

import (
	"archive/tar"
	"io"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
	"github.com/google/go-cmp/cmp/cmpopts"
	"github.com/stretchr/testify/assert"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/server/storage"
)

var files = []storage.ProjectFile{
	{
		Path: "somefile.txt",
		Data: []byte("somecontents"),
	},
	{
		Path: "somedir/anotherfile.txt",
		Data: []byte("anotherfilescontents"),
	},
	{
		Path: "somedir/filtered.txt",
		Data: []byte("filteredcontents"),
	},
	{
		Path: "rootfiltered.txt",
		Data: []byte("anotherfilntents"),
	},
	{
		Path: ".hidden",
		Data: []byte("hiddenfile"),
	},
}

func getFilesInTar(buf io.Reader) ([]storage.ProjectFile, error) {
	fsInTar := []storage.ProjectFile{}
	err := ReadFromTar(buf, func(hdr *tar.Header, contents []byte) error {
		fsInTar = append(fsInTar, storage.ProjectFile{
			Path: hdr.Name,
			Data: contents,
		})
		return nil
	})
	if err != nil {
		return nil, err
	}

	return fsInTar, nil
}

func TestTarDirectory(t *testing.T) {
	testingDir, err := dirtools.MkAndPopulateDirTemp("mytempdir", files)
	if err != nil {
		t.Error(err)
	}

	buf, err := TarDirectory(testingDir, []string{"somedir/fi*", "rootfiltered.txt", ".hidden"})
	if err != nil {
		t.Error(err)
	}

	fsInTar, err := getFilesInTar(buf)
	if err != nil {
		t.Error(err)
	}

	filterFiles := files[0:2]

	assert.ElementsMatch(t, filterFiles, fsInTar)
	err = os.RemoveAll(testingDir)
	assert.NoError(t, err)
}

func TestNegativeValidateDirEntry(t *testing.T) {
	filePath := "d_ir/.hidden"
	filters := []string{"*/.hidden"}
	valid, err := validateDirEntry(filePath, filters)
	assert.NoError(t, err)
	assert.False(t, valid)
}

func TestTarFiles(t *testing.T) {
	buf, err := TarFiles(files)
	if err != nil {
		t.Fatal(err)
	}

	fsInTar, err := getFilesInTar(buf)
	if err != nil {
		t.Error(err)
	}

	less := func(a string, b string) bool { return a < b }
	diff := cmp.Diff(fsInTar, files, cmpopts.SortSlices(less))
	if diff != "" {
		t.Errorf("expected tar to contain the same elements as files, but got diff: %s", diff)
	}
}
