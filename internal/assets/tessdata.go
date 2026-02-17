package assets

import (
	_ "embed"
	"log"
	"os"
	"path/filepath"
)

//go:embed tessdata/eng.traineddata
var engTrainedData []byte

// EnsureTessdata extracts the embedded eng.traineddata to the user's
// application-data directory and returns the path to the tessdata folder.
// On Windows this is typically %APPDATA%\Sqyre\tessdata.
// If the file already exists on disk the extraction is skipped.
// Returns "" if the extraction fails (caller should fall back to system tessdata).
func EnsureTessdata() string {
	dir := tessdataDir()
	if dir == "" {
		return ""
	}
	dest := filepath.Join(dir, "eng.traineddata")

	if _, err := os.Stat(dest); err == nil {
		return dir
	}

	if err := os.MkdirAll(dir, 0755); err != nil {
		log.Printf("EnsureTessdata: could not create directory %s: %v", dir, err)
		return ""
	}
	if err := os.WriteFile(dest, engTrainedData, 0644); err != nil {
		log.Printf("EnsureTessdata: could not write %s: %v", dest, err)
		return ""
	}
	log.Printf("EnsureTessdata: extracted eng.traineddata to %s", dest)
	return dir
}

// tessdataDir returns the tessdata directory inside the app data folder.
func tessdataDir() string {
	// Prefer APPDATA (Windows); fall back to user home directory.
	base := os.Getenv("APPDATA")
	if base == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			log.Printf("EnsureTessdata: cannot determine home directory: %v", err)
			return ""
		}
		base = home
	}
	return filepath.Join(base, "Sqyre", "tessdata")
}
