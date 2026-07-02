package capture

import "testing"

func TestDiagnosticsModeEnvOverride(t *testing.T) {
	t.Setenv("SQYRE_OVERLAY_DIAG", "1")
	t.Setenv("SQYRE_OVERLAY_DIAGNOSTICS", "off")
	if got := diagnosticsMode(); got != DiagnosticsStartup {
		t.Fatalf("got %q, want %q", got, DiagnosticsStartup)
	}
}

func TestDiagnosticsModeFromVariable(t *testing.T) {
	t.Setenv("SQYRE_OVERLAY_DIAG", "")
	t.Setenv("SQYRE_OVERLAY_DIAGNOSTICS", "always")
	if got := diagnosticsMode(); got != DiagnosticsAlways {
		t.Fatalf("got %q, want %q", got, DiagnosticsAlways)
	}
}
