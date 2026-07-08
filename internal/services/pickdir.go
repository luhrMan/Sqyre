package services

import (
	"errors"
	"fmt"
	"os/exec"
	"runtime"
	"strings"
)

// ErrNoDirectoryChooser reports that no native directory chooser is available on
// this platform, so callers should fall back to manual path entry.
var ErrNoDirectoryChooser = errors.New("no native directory chooser available")

// PickDirectory opens the platform's native directory chooser starting at
// startDir and returns the selected absolute path. ok is false when the user
// cancels. It returns ErrNoDirectoryChooser when no supported chooser is
// installed. This shells out to the OS so it does not depend on Fyne's file
// dialog, which crashes with our custom theme (Fyne 2.7.4 richtext bug).
func PickDirectory(title, startDir string) (path string, ok bool, err error) {
	switch runtime.GOOS {
	case "darwin":
		return pickDirDarwin(title, startDir)
	case "windows":
		return pickDirWindows(title, startDir)
	default:
		return pickDirLinux(title, startDir)
	}
}

func pickDirLinux(title, startDir string) (string, bool, error) {
	if bin, lookErr := exec.LookPath("zenity"); lookErr == nil {
		args := []string{"--file-selection", "--directory", "--title=" + title}
		if startDir != "" {
			args = append(args, "--filename="+ensureTrailingSep(startDir))
		}
		return runChooser(bin, args...)
	}
	if bin, lookErr := exec.LookPath("kdialog"); lookErr == nil {
		start := startDir
		if start == "" {
			start = "."
		}
		return runChooser(bin, "--getexistingdirectory", start, "--title", title)
	}
	return "", false, ErrNoDirectoryChooser
}

func pickDirDarwin(title, startDir string) (string, bool, error) {
	script := `set prompt to "` + escapeAppleScript(title) + `"` + "\n"
	if startDir != "" {
		script += `set d to choose folder with prompt prompt default location POSIX file "` + escapeAppleScript(startDir) + `"` + "\n"
	} else {
		script += `set d to choose folder with prompt prompt` + "\n"
	}
	script += `POSIX path of d`
	return runChooser("osascript", "-e", script)
}

func pickDirWindows(title, startDir string) (string, bool, error) {
	ps := `Add-Type -AssemblyName System.Windows.Forms; ` +
		`$d = New-Object System.Windows.Forms.FolderBrowserDialog; ` +
		`$d.Description = '` + strings.ReplaceAll(title, "'", "''") + `'; `
	if startDir != "" {
		ps += `$d.SelectedPath = '` + strings.ReplaceAll(startDir, "'", "''") + `'; `
	}
	ps += `if ($d.ShowDialog() -eq 'OK') { [Console]::Out.Write($d.SelectedPath) } else { exit 1 }`
	return runChooser("powershell", "-NoProfile", "-NonInteractive", "-Command", ps)
}

// runChooser executes a native chooser and interprets its exit status: exit 0
// means a selection (trimmed stdout), a clean non-zero exit means the user
// cancelled, and anything else is a real error.
func runChooser(bin string, args ...string) (string, bool, error) {
	out, err := exec.Command(bin, args...).Output()
	if err != nil {
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			return "", false, nil // cancelled or dismissed
		}
		return "", false, fmt.Errorf("run directory chooser: %w", err)
	}
	path := strings.TrimSpace(string(out))
	if path == "" {
		return "", false, nil
	}
	return path, true, nil
}

func ensureTrailingSep(dir string) string {
	if strings.HasSuffix(dir, "/") {
		return dir
	}
	return dir + "/"
}

func escapeAppleScript(s string) string {
	s = strings.ReplaceAll(s, `\`, `\\`)
	return strings.ReplaceAll(s, `"`, `\"`)
}
