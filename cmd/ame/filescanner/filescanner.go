package filescanner

import (
	"archive/tar"
	"bytes"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path"
	"path/filepath"
)

func isFiltered(input string, filters []string) (bool, error) {
	for _, filter := range filters {
		matched, err := filepath.Match(filter, input)
		if err != nil {
			return false, err
		}

		if matched {
			return true, nil
		}
	}

	return false, nil
}

func validateDirEntry(filePath string, filers []string) (bool, error) {
	filtered, err := isFiltered(filePath, filers)
	if err != nil {
		return false, err
	}

	fmt.Println(filePath, filtered)

	return !filtered, nil
}

func TarDirectory(dir string, filters []string) (*bytes.Buffer, error) {
	var buf bytes.Buffer
	tw := tar.NewWriter(&buf)
	err := filepath.WalkDir(dir, func(walkPath string, d fs.DirEntry, err error) error {
		fmt.Println(walkPath, dir, d)
		if d == nil || d.IsDir() {
			return nil
		}

		valid, err := validateDirEntry(path.Join(walkPath), filters)
		if err != nil {
			return err
		}

		if valid {
			writeToTar(tw, walkPath, d)
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

type TarWalk func(*tar.Header, []byte) error

func ReadFromTar(buf io.Reader, tarWalk TarWalk) error {
	tr := tar.NewReader(buf)
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}

		if err != nil {
			return err
		}

		fileContents, err := io.ReadAll(tr)
		if err != nil {
			return err
		}

		err = tarWalk(hdr, fileContents)
		if err != nil {
			return err
		}

	}

	return nil
}
