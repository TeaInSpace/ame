package filescanner

import (
	"archive/tar"
	"io"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/server/storage"
)

const testingDir = "filescanner_test_dir"

func TestTarDirectory(t *testing.T) {
	files := []storage.ProjectFile{
		{
			testingDir + "/somefile.txt",
			[]byte("somecontents"),
		},
		{
			testingDir + "/somedir/anotherfile.txt",
			[]byte("anotherfilescontents"),
		},
		{
			testingDir + "/somedir/filtered.txt",
			[]byte("filteredcontents"),
		},
		{
			testingDir + "/rootfiltered.txt",
			[]byte("anotherfilntents"),
		},
		{
			testingDir + "/.hidden",
			[]byte("hiddenfile"),
		},
	}

	err := dirtools.PopulateDir(".", files)
	assert.NoError(t, err)

	buf, err := TarDirectory(testingDir, []string{testingDir + "/somedir/fi*", "*/rootfiltered.txt", "*/.hidden"})
	assert.NoError(t, err)

	fsInTar := []storage.ProjectFile{}
	tr := tar.NewReader(buf)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		assert.NoError(t, err)

		fileContents, err := io.ReadAll(tr)
		assert.NoError(t, err)

		fsInTar = append(fsInTar, storage.ProjectFile{
			Path: hdr.Name,
			Data: fileContents,
		})
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
