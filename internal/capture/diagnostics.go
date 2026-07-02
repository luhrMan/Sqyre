package capture

import (
	"os"
	"strings"
)

const (
	DiagnosticsOff     = "off"
	DiagnosticsStartup = "startup"
	DiagnosticsAlways  = "always"
)

func diagnosticsMode() string {
	if os.Getenv("SQYRE_OVERLAY_DIAG") == "1" {
		return DiagnosticsStartup
	}
	raw := strings.TrimSpace(os.Getenv("SQYRE_OVERLAY_DIAGNOSTICS"))
	switch strings.ToLower(raw) {
	case DiagnosticsStartup:
		return DiagnosticsStartup
	case DiagnosticsAlways:
		return DiagnosticsAlways
	default:
		return DiagnosticsOff
	}
}

func diagnosticsEnabled(mode string) bool {
	return mode == DiagnosticsStartup || mode == DiagnosticsAlways
}

// OverlayDiagnosticsEnabled reports whether overlay capture/window diagnostic logs should run.
func OverlayDiagnosticsEnabled() bool {
	return diagnosticsEnabled(diagnosticsMode())
}
