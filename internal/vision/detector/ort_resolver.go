package detector

import (
	"os"
	"path/filepath"

	"Sqyre/internal/config"
)

// ResolvedORTLibrary returns libonnxruntime for the current onnxruntime_go version.
func ResolvedORTLibrary() string {
	if p := os.Getenv(envORTLibPath); p != "" && fileExists(p) {
		return p
	}
	for _, p := range []string{
		filepath.Join(config.GetSqyreDir(), "lib", "libonnxruntime.so"),
	} {
		if fileExists(p) {
			return p
		}
	}
	for _, name := range []string{
		"libonnxruntime.so",
		"libonnxruntime.so.1",
		"/usr/lib/libonnxruntime.so",
		"/usr/local/lib/libonnxruntime.so",
	} {
		if _, err := os.Stat(name); err == nil {
			return name
		}
	}
	return ""
}

// PrepareWorkerEnv sets SQUIRE_ORT_LIB and SQUIRE_VISION_MODEL_DIR when unset.
func PrepareWorkerEnv() {
	if os.Getenv(envORTLibPath) == "" {
		if lib := ResolvedORTLibrary(); lib != "" {
			_ = os.Setenv(envORTLibPath, lib)
		}
	}
	if os.Getenv(envModelsDir) == "" {
		if dir := resolvedModelsDir(); dir != "" {
			_ = os.Setenv(envModelsDir, dir)
		}
	}
}
