package config

import "os"

// IsUITestMode reports whether GUI tests are running (SQUIRE_UI_TEST=1).
// When true, robotgo-backed UI features (mouse position polling, screen info) are stubbed.
func IsUITestMode() bool {
	return os.Getenv("SQUIRE_UI_TEST") == "1"
}
