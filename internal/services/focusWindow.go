package services

import (
	"Sqyre/internal/models/actions"
	"fmt"
	"strings"
)

// ActiveWindows returns open top-level windows with stable executable path and title.
func ActiveWindows() ([]WindowInfo, error) {
	return listOpenWindows()
}

// RunFocusWindow activates the window identified by executable path and title.
func RunFocusWindow(a *actions.FocusWindow) error {
	path := strings.TrimSpace(a.ProcessPath)
	title := strings.TrimSpace(a.WindowTitle)
	if path == "" {
		return fmt.Errorf("focus window: no executable path set")
	}
	if title == "" {
		return fmt.Errorf("focus window: no window title set")
	}
	if err := activateWindow(path, title); err != nil {
		return fmt.Errorf("focus window %q (%s): %w", title, path, err)
	}
	return nil
}
