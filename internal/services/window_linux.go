//go:build linux

package services

import (
	"fmt"
	"strings"
	"unicode/utf8"

	"github.com/go-vgo/robotgo"
	"github.com/robotn/xgbutil"
	"github.com/robotn/xgbutil/ewmh"
)

func listOpenWindows() ([]WindowInfo, error) {
	xu, err := xgbutil.NewConn()
	if err != nil {
		return nil, fmt.Errorf("connect display: %w", err)
	}

	windows, err := ewmh.ClientListGet(xu)
	if err != nil {
		return nil, fmt.Errorf("list windows: %w", err)
	}

	out := make([]WindowInfo, 0, len(windows))
	seen := make(map[string]struct{}, len(windows))
	for _, w := range windows {
		title, err := ewmh.WmNameGet(xu, w)
		if err != nil || strings.TrimSpace(title) == "" {
			continue
		}
		if !utf8.ValidString(title) {
			continue
		}

		pidU, err := ewmh.WmPidGet(xu, w)
		if err != nil || pidU == 0 {
			continue
		}

		pid := int(pidU)
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
	xu, err := xgbutil.NewConn()
	if err != nil {
		return fmt.Errorf("connect display: %w", err)
	}

	windows, err := ewmh.ClientListGet(xu)
	if err != nil {
		return fmt.Errorf("list windows: %w", err)
	}

	for _, w := range windows {
		title, err := ewmh.WmNameGet(xu, w)
		if err != nil {
			continue
		}
		pidU, err := ewmh.WmPidGet(xu, w)
		if err != nil || pidU == 0 {
			continue
		}
		path, _ := robotgo.FindPath(int(pidU))
		if !pathsEqual(path, processPath) || !titlesEqual(title, windowTitle) {
			continue
		}
		if err := robotgo.ActivePid(int(w), 1); err != nil {
			return fmt.Errorf("activate window: %w", err)
		}
		return nil
	}
	return fmt.Errorf("no window with title %q from %q", windowTitle, processPath)
}
