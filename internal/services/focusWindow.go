package services

import (
	"Sqyre/internal/desktop"
	"Sqyre/internal/models/actions"
	"fmt"
	"strings"
)

// ActiveWindowNames returns a list of process/window names that can be used with ActiveName.
// The list is suitable for showing in a dropdown; empty names are filtered out.
func ActiveWindowNames() ([]string, error) {
	names, err := desktop.Default.FindWindowNames()
	if err != nil {
		return nil, fmt.Errorf("list windows: %w", err)
	}
	out := make([]string, 0, len(names))
	seen := make(map[string]bool)
	for _, n := range names {
		n = strings.TrimSpace(n)
		if n == "" || seen[n] {
			continue
		}
		seen[n] = true
		out = append(out, n)
	}
	return out, nil
}

// RunFocusWindow activates/focuses the window specified by the action.
func RunFocusWindow(a *actions.FocusWindow) error {
	target := strings.TrimSpace(a.WindowTarget)
	if target == "" {
		return fmt.Errorf("focus window: no window target set")
	}
	if err := desktop.Default.ActiveWindowByName(target); err != nil {
		return fmt.Errorf("focus window %q: %w", target, err)
	}
	return nil
}
