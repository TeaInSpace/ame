package storage

import (
	"context"
	"testing"

	"github.com/brianvoe/gofakeit/v6"
	"github.com/stretchr/testify/assert"
)

const (
	testDataDir    = "test_data"
	testBucketName = "testbucket"
)

func TestUploadAndDownloadMultipleFiles(t *testing.T) {
	ctx := context.Background()

	s3Client, err := CreateS3ClientForLocalStorage(ctx)
	assert.NoError(t, err)

	storeage := NewS3Storage(*s3Client, testBucketName)
	storeage.ClearStorage(ctx)
	err = storeage.PrepareStorage(ctx)
	assert.NoError(t, err)

	testProjectDir := "myproject"

	testFiles := []ProjectFile{
		{
			Data: []byte(gofakeit.Name()),
			Path: "somedir/somefile",
		},
		{
			Data: []byte(gofakeit.Name()),
			Path: "anotherfile.txt",
		},
		{
			Data: []byte(gofakeit.Name()),
			Path: ".gitignore",
		},
		{
			Data: []byte(gofakeit.Name()),
			Path: "somedir/somedeepdir/deepfile.go",
		},
	}

	contents, err := storeage.DownloadFiles(ctx, "")
	assert.Empty(t, contents)

	for _, projectFile := range testFiles {
		err = storeage.StoreFileInProject(ctx, testProjectDir, projectFile)
		assert.NoError(t, err)
	}

	assert.NoError(t, err)

	files, err := storeage.DownloadFiles(ctx, testProjectDir)
	assert.NoError(t, err)
	assert.ElementsMatch(t, testFiles, files)
	err = storeage.ClearStorage(ctx)
	assert.NoError(t, err)
}
