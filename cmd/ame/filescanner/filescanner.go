package filescanner

import (
	"archive/tar"
	"bytes"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"strings"

	"teainspace.com/ame/server/storage"
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

	return !filtered, nil
}

func TarDirectory(dir string, filters []string) (*bytes.Buffer, error) {
	var buf bytes.Buffer
	tw := tar.NewWriter(&buf)
	err := filepath.WalkDir(dir, func(walkPath string, d fs.DirEntry, err error) error {
		// The err parameter indidcates that something whent wrong the walking
		// the directory, as we don't know how to handle that case at the moment
		// the error is returned and the directory walk will stop.
		// see the fs.WalkFunc documentation for further details.
		if err != nil {
			return err
		}

		if d == nil || d.IsDir() {
			return nil
		}

		relativePath := strings.Replace(walkPath, dir+"/", "", 1)
		valid, err := validateDirEntry(relativePath, filters)
		if err != nil {
			return err
		}

		if valid {
			writePathToTar(tw, walkPath, relativePath, d)
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

func writePathToTar(tw *tar.Writer, path string, relativePath string, d fs.DirEntry) error {
	contents, err := os.ReadFile(path)
	if err != nil {
		return err
	}

	fInfo, err := d.Info()
	if err != nil {
		return err
	}

	return writeToTar(tw, contents, relativePath, int64(fInfo.Mode()), fInfo.Size())
}

func writeToTar(tw *tar.Writer, contents []byte, relativePath string, fileMode int64, fileSize int64) error {
	hdr := tar.Header{
		Name: relativePath,
		Mode: fileMode,
		Size: fileSize,
	}
	err := tw.WriteHeader(&hdr)
	if err != nil {
		return err
	}

	_, err = tw.Write(contents)
	if err != nil {
		return err
	}
	return nil
}

func writeFileToTar(tw *tar.Writer, f storage.ProjectFile) error {
	return writeToTar(tw, f.Data, f.Path, 0o777, int64(len(f.Data)))
}

// TarFiles archives the files in the files parameter as a tar file and returns
// a pointer to the buffer containing the archive.
func TarFiles(files []storage.ProjectFile) (*bytes.Buffer, error) {
	var buf bytes.Buffer
	tw := tar.NewWriter(&buf)

	for _, f := range files {
		err := writeFileToTar(tw, f)
		if err != nil {
			return nil, err
		}
	}

	return &buf, nil
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
