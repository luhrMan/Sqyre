package detector

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
)

// appImageLinker returns the bundled dynamic linker when running inside an AppImage.
// AppImage binaries use a relative PT_INTERP (lib64/ld-linux-x86-64.so.2) that only
// AppRun resolves; child processes must invoke the linker explicitly.
func appImageLinker() string {
	appdir := os.Getenv("APPDIR")
	if appdir == "" {
		return ""
	}
	linker := filepath.Join(appdir, "runtime", "default", "lib64", "ld-linux-x86-64.so.2")
	if fileExists(linker) {
		return linker
	}
	return ""
}

func workerCommand(ctx context.Context, workerPath string, args ...string) *exec.Cmd {
	if linker := appImageLinker(); linker != "" {
		allArgs := append([]string{workerPath}, args...)
		if ctx != nil {
			return exec.CommandContext(ctx, linker, allArgs...)
		}
		return exec.Command(linker, allArgs...)
	}
	if ctx != nil {
		return exec.CommandContext(ctx, workerPath, args...)
	}
	return exec.Command(workerPath, args...)
}
