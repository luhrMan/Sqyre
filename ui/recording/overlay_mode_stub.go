//go:build (!linux || wayland) && !windows

package recording

func useVirtualDesktopOverlay() bool { return false }
