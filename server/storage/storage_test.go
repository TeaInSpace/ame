package storage

import (
	"context"
	"os"
	"testing"

	"github.com/brianvoe/gofakeit/v6"
	"github.com/google/go-cmp/cmp"
	"github.com/google/go-cmp/cmp/cmpopts"
	"github.com/stretchr/testify/assert"
	"teainspace.com/ame/internal/testcfg"
)

var (
	ctx     context.Context
	testCfg testcfg.TestEnvConfig
)

var testFiles = []ProjectFile{
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

func TestMain(m *testing.M) {
	ctx = context.Background()
	testCfg = testcfg.TestEnv()
	os.Exit(m.Run())
}

func TestUploadAndDownloadMultipleFiles(t *testing.T) {
	s3Client, err := CreateS3ClientForLocalStorage(ctx, testCfg.ObjectStorageEndpoint)
	assert.NoError(t, err)

	storeage := NewS3Storage(*s3Client, testCfg.BucketName)
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

func TestCanDownloadTaskArtifacts(t *testing.T) {
	taskName := "sometask"

	store, err := SetupStoreage(ctx, testCfg.BucketName, testCfg.ObjectStorageEndpoint)
	if err != nil {
		t.Fatal(err)
	}

	err = store.StoreArtifacts(ctx, taskName, testFiles)
	if err != nil {
		t.Fatal(err)
	}

	artifacts, err := store.DownloadArtifacts(ctx, taskName)
	if err != nil {
		t.Fatal(err)
	}

	diff := cmp.Diff(artifacts, testFiles, cmpopts.SortSlices(FileCmp))
	if diff != "" {
		t.Errorf("expected all uploaded artifacts to be downloaded, but got diffL %s", diff)
	}
}
