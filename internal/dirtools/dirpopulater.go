package dirtools

import (
	"fmt"
	"os"
	"path"

	"teainspace.com/ame/server/storage"
)

func PopulateDir(dir string, files []storage.ProjectFile) error {
	err := os.MkdirAll(dir, 0o755)
	if err != nil {
		return err
	}

	for _, f := range files {
		dir := fmt.Sprintf("%s/%s", dir, path.Dir(f.Path))
		if dir != "" {
			err := os.MkdirAll(dir, 0o755)
			if err != nil {
				return err
			}
		}
		err = os.WriteFile(dir+"/"+path.Base(f.Path), f.Data, 0o644)
		if err != nil {
			return err
		}
	}

	return nil
}
