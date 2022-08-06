package filescanner

import (
	"archive/tar"
	"bytes"
	"path"

	"teainspace.com/ame/server/storage"
)

type WalkProjectFunc func(p storage.ProjectFile) error

type ProjectPackaging interface {
	PackageProject(dir string) (bytes.Buffer, error)
	WalkProject(data []byte, walkFunc WalkProjectFunc) error
}

type ProjectPackagingConfig struct {
	name string
}

type TarProjectPacker struct {
	cfg ProjectPackagingConfig
}

func NewTarProjectPacker(name string) *TarProjectPacker {
	return &TarProjectPacker{
		ProjectPackagingConfig{
			name: name,
		},
	}
}

func (p *TarProjectPacker) PackageProject(dir string, filters []string) (*bytes.Buffer, error) {
	return TarDirectory(dir, filters)
}

func (p *TarProjectPacker) WalkProject(data *bytes.Buffer, walkFunc WalkProjectFunc) error {
	return ReadFromTar(data, func(h *tar.Header, b []byte) error {
		return walkFunc(storage.ProjectFile{
			Path: path.Join(p.cfg.name, h.Name),
			Data: b,
		})
	})
}
