package assets

import (
	"embed"
	"io/fs"
	"log"
	"os"
	"path/filepath"
)

//go:embed tessdata
var tessFS embed.FS

// EnsureTessdata materialises the embedded tessdata filesystem into a
// RAM-backed tmpfs directory (/dev/shm on Linux) so the gosseract C
// library can read it by path without any persistent disk I/O.
// Falls back to the OS temp directory when /dev/shm is unavailable.
// Returns the directory containing eng.traineddata, or "" on failure.
func EnsureTessdata() string {
	dir := tessdataTmpDir()
	dest := filepath.Join(dir, "eng.traineddata")

	if _, err := os.Stat(dest); err == nil {
		return dir
	}

	if err := os.MkdirAll(dir, 0755); err != nil {
		log.Printf("EnsureTessdata: could not create directory %s: %v", dir, err)
		return ""
	}

	data, err := fs.ReadFile(tessFS, "tessdata/eng.traineddata")
	if err != nil {
		log.Printf("EnsureTessdata: could not read embedded tessdata: %v", err)
		return ""
	}

	if err := os.WriteFile(dest, data, 0644); err != nil {
		log.Printf("EnsureTessdata: could not write %s: %v", dest, err)
		return ""
	}
	log.Printf("EnsureTessdata: materialised eng.traineddata to %s", dest)
	return dir
}

// tessdataTmpDir returns a RAM-backed directory for tessdata extraction.
// Prefers /dev/shm (Linux tmpfs) so the data never touches persistent
// storage; falls back to the OS temp directory on Windows / other platforms.
func tessdataTmpDir() string {
	const shmDir = "/dev/shm"
	if info, err := os.Stat(shmDir); err == nil && info.IsDir() {
		return filepath.Join(shmDir, "sqyre-tessdata")
	}
	return filepath.Join(os.TempDir(), "sqyre-tessdata")
}
