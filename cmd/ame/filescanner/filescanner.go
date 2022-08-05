package filescanner

import (
	"archive/tar"
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
)

func TarDirectory(dir string) (*bytes.Buffer, error) {
	var buf bytes.Buffer
	tw := tar.NewWriter(&buf)
	err := filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
		if !d.IsDir() {
			writeToTar(tw, path, d)
		}

		return nil
	})
	if err != nil {
		return nil, err
	}

	err = tw.Close()
	if err != nil {
		return nil, err
	}

	return &buf, err
}

func writeToTar(tw *tar.Writer, path string, d fs.DirEntry) error {
	fInfo, err := d.Info()
	if err != nil {
		return err
	}
	hdr := tar.Header{
		Name: path,
		Mode: int64(fInfo.Mode()),
		Size: fInfo.Size(),
	}
	err = tw.WriteHeader(&hdr)
	if err != nil {
		return err
	}

	contents, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	_, err = tw.Write(contents)
	if err != nil {
		return err
	}
	return nil
}
