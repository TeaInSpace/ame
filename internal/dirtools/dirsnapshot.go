package dirtools

import (
	"crypto/md5"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// FileSnapshot reprents a snapshot of a file at some relative path
// to a project root. The snapshot takes the form of an md5 hash, in
// the Checksum property.
type FileSnapshot struct {
	RelativePath string
	Checksum     string
}

// SnapDir calculates the md5 checksums for every file in root.
// An array of FileSnapShots is returned containing the checksums for
// every file,
func SnapDir(root string) ([]FileSnapshot, error) {
	snapshots := []FileSnapshot{}
	err := filepath.WalkDir(root, func(walkPath string, d fs.DirEntry, err error) error {
		// The err parameter indidcates that something whent wrong the walking
		// the directory, as we don't know how to handle that case at the moment
		// the error is returned and the directory walk will stop.
		// see the fs.WalkFunc documentation for further details.
		if err != nil {
			return err
		}

		if d.IsDir() {
			return nil
		}

		checkSum, err := hashFile(walkPath)
		if err != nil {
			return err
		}

		fSnap := FileSnapshot{
			RelativePath: strings.Replace(walkPath, root+"/", "", 1),
			Checksum:     checkSum,
		}

		snapshots = append(snapshots, fSnap)
		return nil
	})
	if err != nil {
		return nil, err
	}

	return snapshots, nil
}

func hashFile(path string) (string, error) {
	f, err := os.OpenFile(path, os.O_RDONLY, 0o644)
	if err != nil {
		return "", err
	}

	h := md5.New()
	_, err = io.Copy(h, f)
	if err != nil {
		return "", err
	}

	return string(h.Sum(nil)), nil
}

// SnapshotDiff finds files in y which are not in x and returns them.
func SnapshotDiff(x []FileSnapshot, y []FileSnapshot) []FileSnapshot {
	diff := []FileSnapshot{}

	for _, yfSnap := range y {
		foundMatch := false
		for _, xfSnap := range x {
			if xfSnap.RelativePath == yfSnap.RelativePath &&
				xfSnap.Checksum == yfSnap.Checksum {
				foundMatch = true
				break
			}
		}

		if !foundMatch {
			diff = append(diff, yfSnap)
		}
	}

	return diff
}
