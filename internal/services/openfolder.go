package services

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"syscall"

	"Sqyre/internal/config"
)

// OpenPathInFileManager opens path in the platform file manager.
func OpenPathInFileManager(path string) error {
	if path == "" {
		return fmt.Errorf("open folder: empty path")
	}
	if _, err := os.Stat(path); err != nil {
		return fmt.Errorf("open folder %q: %w", path, err)
	}

	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "windows":
		cmd = exec.Command("explorer", path)
	case "darwin":
		cmd = exec.Command("open", path)
	default:
		cmd = exec.Command("xdg-open", path)
	}
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("open folder %q: %w", path, err)
	}
	return nil
}

// OpenSqyreDir ensures ~/.sqyre exists and opens it in the file manager.
func OpenSqyreDir() error {
	dir := config.GetSqyreDir()
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create sqyre directory: %w", err)
	}
	return OpenPathInFileManager(dir)
}

// MoveDir moves src to dst, falling back to a recursive copy when the two paths
// live on different filesystems (os.Rename returns EXDEV). dst must not already
// exist so we never merge into or overwrite unrelated data.
func MoveDir(src, dst string) error {
	if src == dst {
		return nil
	}
	if _, err := os.Stat(dst); err == nil {
		return fmt.Errorf("move %q to %q: destination already exists", src, dst)
	} else if !os.IsNotExist(err) {
		return fmt.Errorf("move %q to %q: %w", src, dst, err)
	}

	if err := os.MkdirAll(filepath.Dir(dst), 0755); err != nil {
		return fmt.Errorf("create parent of %q: %w", dst, err)
	}

	if err := os.Rename(src, dst); err == nil {
		return nil
	} else if !errors.Is(err, syscall.EXDEV) {
		return fmt.Errorf("move %q to %q: %w", src, dst, err)
	}

	// Cross-device: copy then remove the source.
	if err := copyTree(src, dst); err != nil {
		return fmt.Errorf("copy %q to %q: %w", src, dst, err)
	}
	if err := os.RemoveAll(src); err != nil {
		return fmt.Errorf("remove old directory %q: %w", src, err)
	}
	return nil
}

// copyTree recursively copies the directory tree rooted at src into dst.
func copyTree(src, dst string) error {
	return filepath.Walk(src, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(src, path)
		if err != nil {
			return err
		}
		target := filepath.Join(dst, rel)

		if info.IsDir() {
			return os.MkdirAll(target, info.Mode().Perm())
		}
		return copyFile(path, target, info.Mode().Perm())
	})
}

func copyFile(src, dst string, perm os.FileMode) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	out, err := os.OpenFile(dst, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, perm)
	if err != nil {
		return err
	}
	if _, err := io.Copy(out, in); err != nil {
		out.Close()
		return err
	}
	return out.Close()
}
