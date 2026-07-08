package detector

import (
	"os"
	"path/filepath"
	"sync"

	"Sqyre/internal/config"
)

// RuntimeConfig holds user settings for semantic vision (worker path, models dir).
type RuntimeConfig struct {
	WorkerPath string // empty = auto-detect sibling sqyre-vision
	ModelsDir  string // empty = ~/.sqyre/models (passed to worker)
}

var (
	runtimeCfgMu sync.RWMutex
	runtimeCfg   RuntimeConfig
)

// SetRuntimeConfig updates detector routing from application settings.
func SetRuntimeConfig(c RuntimeConfig) {
	runtimeCfgMu.Lock()
	runtimeCfg = c
	runtimeCfgMu.Unlock()
	resetRemoteWorkers()
}

// RuntimeConfigSnapshot returns the current runtime configuration.
func RuntimeConfigSnapshot() RuntimeConfig {
	runtimeCfgMu.RLock()
	defer runtimeCfgMu.RUnlock()
	return runtimeCfg
}

func resolvedModelsDir() string {
	if c := RuntimeConfigSnapshot(); c.ModelsDir != "" {
		return c.ModelsDir
	}
	if d := os.Getenv(envModelsDir); d != "" {
		return d
	}
	return config.GetModelsPath()
}

// ResolveWorkerPath returns the sqyre-vision binary path, or "" if unavailable.
func ResolveWorkerPath() string {
	if c := RuntimeConfigSnapshot(); c.WorkerPath != "" {
		if fileExists(c.WorkerPath) {
			return c.WorkerPath
		}
		// Stale preference path — fall through to auto-detect.
	}
	if p := os.Getenv("SQUIRE_VISION_WORKER"); p != "" && fileExists(p) {
		return p
	}
	if exe, err := os.Executable(); err == nil {
		sibling := filepath.Join(filepath.Dir(exe), "sqyre-vision")
		if fileExists(sibling) {
			return sibling
		}
	}
	if p, err := execLookPath("sqyre-vision"); err == nil {
		return p
	}
	return ""
}

func fileExists(path string) bool {
	st, err := os.Stat(path)
	return err == nil && !st.IsDir()
}

// execLookPath is set in resolver_*.go init.
var execLookPath func(name string) (string, error)
