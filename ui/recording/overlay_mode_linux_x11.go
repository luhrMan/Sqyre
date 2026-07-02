//go:build linux && !wayland

package recording

// Per-monitor overlay windows avoid unified virtual-desktop composition issues on
// asymmetric multi-monitor X11 layouts (mixed resolutions and origins).
func useVirtualDesktopOverlay() bool { return false }
