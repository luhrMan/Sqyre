//go:build !windows

package services

import (
	"fmt"
	"os"
	"path/filepath"
	"syscall"
)

// relaunchExec replaces the current process image (same PID). Inside an AppImage
// this keeps the runtime's FUSE mount alive, so the app restarts without a
// second mount and without orphaning the old runtime process.
func relaunchExec() error {
	target, argv := reexecTarget()
	if err := syscall.Exec(target, argv, os.Environ()); err != nil {
		return fmt.Errorf("re-exec %q: %w", target, err)
	}
	return nil
}

// reexecTarget picks what to exec. Inside an AppImage we re-run $APPDIR/AppRun
// so the bundled linker and library paths are configured exactly as on first
// launch, reusing the already-mounted squashfs. Otherwise we re-run the binary.
func reexecTarget() (string, []string) {
	if appDir := os.Getenv("APPDIR"); appDir != "" {
		appRun := filepath.Join(appDir, "AppRun")
		if st, err := os.Stat(appRun); err == nil && !st.IsDir() {
			return appRun, append([]string{appRun}, os.Args[1:]...)
		}
	}
	exe, err := os.Executable()
	if err != nil {
		return os.Args[0], os.Args
	}
	return exe, os.Args
}
