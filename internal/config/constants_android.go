//go:build android

package config

import (
	"log"
	"os"
	"path/filepath"
	"strings"
)

// androidAppBaseDir is computed once at init from TMPDIR/TempDir so we use a
// writable path even when later called from threads where env might differ.
var androidAppBaseDir string

func init() {
	// Use app internal storage so config dirs are writable without storage permission.
	// os.UserHomeDir() on Android returns /sdcard which is not writable by default.
	// Only accept paths under /data/user/0/ or /data/data/ (app-specific); avoid
	// /data or /data/local/tmp which are not writable by the app.
	for _, tmp := range []string{os.Getenv("TMPDIR"), os.TempDir()} {
		if tmp == "" {
			continue
		}
		appRoot := filepath.Clean(filepath.Join(tmp, "..", ".."))
		if appRoot != "" && appRoot != "." &&
			(strings.HasPrefix(appRoot, "/data/user/0/") || strings.HasPrefix(appRoot, "/data/data/")) {
			androidAppBaseDir = appRoot
			break
		}
	}
	if androidAppBaseDir == "" {
		// TMPDIR/TempDir not set or not under app storage (e.g. on some devices).
		// Use the standard app data path for our package (primary user).
		androidAppBaseDir = "/data/user/0/com.sqyre.app"
		if err := os.MkdirAll(androidAppBaseDir, 0755); err != nil {
			// Fallback to UserHomeDir only if app path fails (e.g. in tests)
			homeDir, err2 := os.UserHomeDir()
			if err2 != nil {
				log.Fatalf("Could not get config base directory: %v", err)
			}
			androidAppBaseDir = homeDir
		}
	}
	getConfigBaseDirFn = func() string { return androidAppBaseDir }
}
