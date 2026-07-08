//go:build !windows

package detector

import "os/exec"

func init() {
	execLookPath = exec.LookPath
}
