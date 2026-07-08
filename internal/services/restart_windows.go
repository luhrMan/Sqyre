//go:build windows

package services

import (
	"fmt"
	"os"
	"os/exec"
	"syscall"
)

// relaunchExec spawns a fresh detached instance of the current binary in a new
// process group so it is not terminated together with the exiting parent.
func relaunchExec() error {
	exe, err := os.Executable()
	if err != nil {
		return fmt.Errorf("locate executable: %w", err)
	}

	cmd := exec.Command(exe, os.Args[1:]...)
	cmd.Env = os.Environ()
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin
	cmd.SysProcAttr = &syscall.SysProcAttr{CreationFlags: 0x00000200} // CREATE_NEW_PROCESS_GROUP

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("relaunch %q: %w", exe, err)
	}
	return nil
}
