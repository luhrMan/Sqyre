package services

import (
	"path/filepath"
	"runtime"
	"strings"
)

// WindowInfo describes one top-level application window for picker UI and focus.
type WindowInfo struct {
	Title       string
	ProcessName string
	ProcessPath string
}

// Label returns a human-readable line for list display.
func (w WindowInfo) Label() string {
	title := strings.TrimSpace(w.Title)
	if title == "" {
		title = "(untitled)"
	}
	name := strings.TrimSpace(w.ProcessName)
	path := strings.TrimSpace(w.ProcessPath)
	switch {
	case name != "" && path != "":
		return title + "  (" + name + " — " + path + ")"
	case name != "":
		return title + "  (" + name + ")"
	case path != "":
		return title + "  (" + path + ")"
	default:
		return title
	}
}

func pathsEqual(a, b string) bool {
	a = filepath.Clean(strings.TrimSpace(a))
	b = filepath.Clean(strings.TrimSpace(b))
	if a == "" || b == "" {
		return false
	}
	if runtime.GOOS == "windows" {
		return strings.EqualFold(a, b)
	}
	return a == b
}

func titlesEqual(a, b string) bool {
	return strings.TrimSpace(a) == strings.TrimSpace(b)
}
