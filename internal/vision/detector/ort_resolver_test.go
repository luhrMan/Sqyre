package detector

import (
	"os"
	"path/filepath"
	"testing"

	"Sqyre/internal/config"
)

func TestResolvedORTLibrary_prefersEnv(t *testing.T) {
	t.Helper()
	dir := t.TempDir()
	lib := filepath.Join(dir, "libonnxruntime.so")
	if err := os.WriteFile(lib, []byte{0}, 0o644); err != nil {
		t.Fatal(err)
	}
	t.Setenv(envORTLibPath, lib)
	if got := ResolvedORTLibrary(); got != lib {
		t.Fatalf("got %q want %q", got, lib)
	}
}

func TestResolvedORTLibrary_sqyreLibDir(t *testing.T) {
	t.Helper()
	dir := t.TempDir()
	t.Setenv("SQYRE_TEST_MODE", "1")
	t.Setenv("HOME", dir)
	if err := config.InitializeDirectories(); err != nil {
		t.Fatal(err)
	}
	libDir := filepath.Join(config.GetSqyreDir(), "lib")
	if err := os.MkdirAll(libDir, 0o755); err != nil {
		t.Fatal(err)
	}
	lib := filepath.Join(libDir, "libonnxruntime.so")
	if err := os.WriteFile(lib, []byte{0}, 0o644); err != nil {
		t.Fatal(err)
	}
	t.Setenv(envORTLibPath, "")
	if got := ResolvedORTLibrary(); got != lib {
		t.Fatalf("got %q want %q", got, lib)
	}
}
