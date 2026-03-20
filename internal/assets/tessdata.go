package assets

import (
	_ "embed"
	"log"
	"os"
	"path/filepath"
)

// engTrainedData is the English Tesseract language data. Embedded at build time;
// the build fails if internal/assets/tessdata/eng.traineddata is missing.
//go:embed tessdata/eng.traineddata
var engTrainedData []byte

// EngTrainedData returns the embedded English traineddata for in-memory Tesseract init (no disk write).
func EngTrainedData() []byte { return engTrainedData }

// EnsureTessdata extracts language training data from the embedded filesystem
// into a "tessdata" directory and returns the parent path for TESSDATA_PREFIX.
// Tesseract expects TESSDATA_PREFIX to be the parent of the tessdata folder
// (it looks for $TESSDATA_PREFIX/tessdata/eng.traineddata). Prefers the app
// data directory (e.g. %APPDATA%\.sqyre on Windows, ~/.sqyre elsewhere).
// If that is not writable, extracts to a temporary directory. Returns ""
// only if extraction fails entirely (caller may fall back to system tessdata).
// The returned path is absolute and uses forward slashes so Tesseract's C API
// accepts it on all platforms (Windows can fail with backslashes or relative paths).
func EnsureTessdata() string {
	preferred := tessdataDir() // .../tessdata
	if preferred != "" {
		if extractTessdataTo(preferred, true) != "" {
			return normalizeTessdataPrefix(filepath.Dir(preferred))
		}
	}
	// Fallback: extract to temp/tessdata/ so prefix = temp
	tmp, err := os.MkdirTemp("", "sqyre-tessdata-*")
	if err != nil {
		log.Printf("EnsureTessdata: could not create temp dir: %v", err)
		return ""
	}
	tessdataSub := filepath.Join(tmp, "tessdata")
	if extractTessdataTo(tessdataSub, false) != "" {
		log.Printf("EnsureTessdata: using temp tessdata prefix %s", tmp)
		return normalizeTessdataPrefix(tmp)
	}
	_ = os.RemoveAll(tmp)
	return ""
}

// normalizeTessdataPrefix returns an absolute path with forward slashes for
// Tesseract's C API (avoids init -1 on Windows with backslashes or relative paths).
func normalizeTessdataPrefix(dir string) string {
	abs, err := filepath.Abs(dir)
	if err != nil {
		return dir
	}
	return filepath.ToSlash(abs)
}

// extractTessdataTo writes the embedded eng.traineddata into dir and returns dir on success.
// dir is the tessdata directory (eng.traineddata goes in dir/eng.traineddata).
// skipIfExists: when true, skip writing if the file already exists (used for app data dir).
func extractTessdataTo(dir string, skipIfExists bool) string {
	if err := os.MkdirAll(dir, 0755); err != nil {
		log.Printf("EnsureTessdata: could not create directory %s: %v", dir, err)
		return ""
	}
	dest := filepath.Join(dir, "eng.traineddata")
	if skipIfExists {
		if _, err := os.Stat(dest); err == nil {
			return dir
		}
	}
	if err := os.WriteFile(dest, engTrainedData, 0644); err != nil {
		log.Printf("EnsureTessdata: could not write %s: %v", dest, err)
		return ""
	}
	log.Printf("EnsureTessdata: extracted eng.traineddata to %s", dest)
	return dir
}

// tessdataDir returns the tessdata directory inside the app data folder
// (e.g. .../.sqyre/tessdata). Tesseract expects TESSDATA_PREFIX to be the
// parent of this (e.g. .../.sqyre).
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
	return filepath.Join(base, ".sqyre", "tessdata")
}
