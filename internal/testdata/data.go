package testdata

import (
	"github.com/brianvoe/gofakeit/v6"
	"teainspace.com/ame/server/storage"
)

var TestFiles = []storage.ProjectFile{
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

const (
	TestingGitSource    = "https://github.com/jmintb/ame-showcase.git"
	TestingGitReference = "main"
)
