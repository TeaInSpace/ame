package filescanner

import (
	"archive/tar"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/server/storage"
)

func TestTarDirectory(t *testing.T) {
	files := []storage.ProjectFile{
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

	testingDir, err := dirtools.MkAndPopulateDirTemp("mytempdir", files)
	if err != nil {
		t.Error(err)
	}

	buf, err := TarDirectory(testingDir, []string{"somedir/fi*", "rootfiltered.txt", ".hidden"})
	if err != nil {
		t.Error(err)
	}

	fsInTar := []storage.ProjectFile{}
	err = ReadFromTar(buf, func(hdr *tar.Header, contents []byte) error {
		fsInTar = append(fsInTar, storage.ProjectFile{
			Path: hdr.Name,
			Data: contents,
		})
		return nil
	})

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
