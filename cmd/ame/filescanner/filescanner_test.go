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
	}

	err := dirtools.PopulateDir(".", files)
	assert.NoError(t, err)

	buf, err := TarDirectory(testingDir)
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

	assert.ElementsMatch(t, files, fsInTar)
	err = os.RemoveAll(testingDir)
	assert.NoError(t, err)
}
