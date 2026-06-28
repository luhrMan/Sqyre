//go:build darwin

package services

import (
	"fmt"

	"github.com/go-vgo/robotgo"
)

func listOpenWindows() ([]WindowInfo, error) {
	pids, err := robotgo.Pids()
	if err != nil {
		return nil, fmt.Errorf("list processes: %w", err)
	}

	out := make([]WindowInfo, 0, len(pids))
	seen := make(map[string]struct{}, len(pids))
	for _, pid := range pids {
		title := robotgo.GetTitle(pid)
		if title == "" {
			continue
		}
		name, _ := robotgo.FindName(pid)
		path, _ := robotgo.FindPath(pid)
		key := fmt.Sprintf("%d:%s:%s", pid, path, title)
		if _, ok := seen[key]; ok {
			continue
		}
		seen[key] = struct{}{}
		out = append(out, WindowInfo{
			Title:       title,
			ProcessName: name,
			ProcessPath: path,
		})
	}
	return out, nil
}

func activateWindow(processPath, windowTitle string) error {
	pids, err := robotgo.Pids()
	if err != nil {
		return fmt.Errorf("list processes: %w", err)
	}
	for _, pid := range pids {
		title := robotgo.GetTitle(pid)
		if !titlesEqual(title, windowTitle) {
			continue
		}
		path, _ := robotgo.FindPath(pid)
		if !pathsEqual(path, processPath) {
			continue
		}
		if err := robotgo.ActivePid(pid); err != nil {
			return fmt.Errorf("activate window: %w", err)
		}
		return nil
	}
	return fmt.Errorf("no window with title %q from %q", windowTitle, processPath)
}
