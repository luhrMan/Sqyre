package services

import (
	"fmt"
	"os"
	"os/exec"
	"runtime"

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

// OpenModelsDir ensures ~/.sqyre/models exists and opens it in the file manager.
func OpenModelsDir() error {
	dir := config.GetModelsPath()
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create models directory: %w", err)
	}
	return OpenPathInFileManager(dir)
}
