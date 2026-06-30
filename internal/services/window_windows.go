//go:build windows

package services

import (
	"fmt"
	"syscall"
	"unsafe"

	"github.com/go-vgo/robotgo"
)

var (
	user32                  = syscall.NewLazyDLL("user32.dll")
	procEnumWindows         = user32.NewProc("EnumWindows")
	procIsWindowVisible     = user32.NewProc("IsWindowVisible")
	procGetWindowTextLength = user32.NewProc("GetWindowTextLengthW")
	procGetWindowText       = user32.NewProc("GetWindowTextW")
	procGetWindowThreadPID  = user32.NewProc("GetWindowThreadProcessId")
)

func listOpenWindows() ([]WindowInfo, error) {
	var out []WindowInfo
	seen := make(map[string]struct{})

	cb := syscall.NewCallback(func(hwnd syscall.Handle, _ uintptr) uintptr {
		visible, _, _ := procIsWindowVisible.Call(uintptr(hwnd))
		if visible == 0 {
			return 1
		}

		title := windowText(hwnd)
		if title == "" {
			return 1
		}

		pid := windowPID(hwnd)
		if pid == 0 {
			return 1
		}

		name, _ := robotgo.FindName(pid)
		path, _ := robotgo.FindPath(pid)
		key := fmt.Sprintf("%d:%s:%s", pid, path, title)
		if _, ok := seen[key]; ok {
			return 1
		}
		seen[key] = struct{}{}
		out = append(out, WindowInfo{
			Title:       title,
			ProcessName: name,
			ProcessPath: path,
		})
		return 1
	})

	if r, _, err := procEnumWindows.Call(cb, 0); r == 0 {
		return nil, fmt.Errorf("enum windows: %w", err)
	}
	return out, nil
}

func activateWindow(processPath, windowTitle string) error {
	var matchHwnd syscall.Handle
	found := false

	cb := syscall.NewCallback(func(hwnd syscall.Handle, _ uintptr) uintptr {
		if found {
			return 1
		}
		visible, _, _ := procIsWindowVisible.Call(uintptr(hwnd))
		if visible == 0 {
			return 1
		}
		title := windowText(hwnd)
		if !titlesEqual(title, windowTitle) {
			return 1
		}
		pid := windowPID(hwnd)
		path, _ := robotgo.FindPath(pid)
		if !pathsEqual(path, processPath) {
			return 1
		}
		matchHwnd = hwnd
		found = true
		return 0
	})

	procEnumWindows.Call(cb, 0)
	if !found {
		return fmt.Errorf("no window with title %q from %q", windowTitle, processPath)
	}
	if err := robotgo.ActivePid(int(matchHwnd), 1); err != nil {
		return fmt.Errorf("activate window: %w", err)
	}
	return nil
}

func windowPID(hwnd syscall.Handle) int {
	var pid uint32
	procGetWindowThreadPID.Call(uintptr(hwnd), uintptr(unsafe.Pointer(&pid)))
	return int(pid)
}

func windowText(hwnd syscall.Handle) string {
	n, _, _ := procGetWindowTextLength.Call(uintptr(hwnd))
	if n == 0 {
		return ""
	}
	buf := make([]uint16, n+1)
	procGetWindowText.Call(uintptr(hwnd), uintptr(unsafe.Pointer(&buf[0])), uintptr(len(buf)))
	return syscall.UTF16ToString(buf)
}
