package storage

import (
	"context"
	fmt "fmt"
	"testing"

	"github.com/brianvoe/gofakeit/v6"
	_ "github.com/joho/godotenv/autoload"
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
	err = storeage.PrepareStorage(ctx)
	assert.NoError(t, err)

	testProjectDir := "myproject"

	testFiles := []ProjectFile{
		{
			Data: []byte(gofakeit.Name()),
			Path: fmt.Sprintf("%s/somedir/somefile", testProjectDir),
		},
		{
			Data: []byte(gofakeit.Name()),
			Path: fmt.Sprintf("%s/anotherfile.txt", testProjectDir),
		},
		{
			Data: []byte(gofakeit.Name()),
			Path: fmt.Sprintf("%s/.gitignore", testProjectDir),
		},
		{
			Data: []byte(gofakeit.Name()),
			Path: fmt.Sprintf("%s/somedir/somedeepdir/deepfile.go", testProjectDir),
		},
	}

	contents, err := storeage.DownloadFiles(ctx, "")
	assert.Empty(t, contents)

	for _, projectFile := range testFiles {
		err = storeage.StoreFile(ctx, projectFile)
		assert.NoError(t, err)
	}

	assert.NoError(t, err)

	files, err := storeage.DownloadFiles(ctx, testProjectDir)
	assert.NoError(t, err)
	assert.ElementsMatch(t, testFiles, files)
	err = storeage.ClearStorage(ctx)
	assert.NoError(t, err)
}
