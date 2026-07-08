package detector

import (
	"os"
	"path/filepath"
	"testing"
)

func TestAppImageLinker(t *testing.T) {
	t.Setenv("APPDIR", "")
	if got := appImageLinker(); got != "" {
		t.Fatalf("expected empty linker, got %q", got)
	}

	dir := t.TempDir()
	linkerDir := filepath.Join(dir, "runtime", "default", "lib64")
	if err := os.MkdirAll(linkerDir, 0o755); err != nil {
		t.Fatal(err)
	}
	linker := filepath.Join(linkerDir, "ld-linux-x86-64.so.2")
	if err := os.WriteFile(linker, []byte{0}, 0o755); err != nil {
		t.Fatal(err)
	}
	t.Setenv("APPDIR", dir)
	if got := appImageLinker(); got != linker {
		t.Fatalf("got %q want %q", got, linker)
	}
}

func TestWorkerCommandUsesAppImageLinker(t *testing.T) {
	dir := t.TempDir()
	linkerDir := filepath.Join(dir, "runtime", "default", "lib64")
	if err := os.MkdirAll(linkerDir, 0o755); err != nil {
		t.Fatal(err)
	}
	linker := filepath.Join(linkerDir, "ld-linux-x86-64.so.2")
	if err := os.WriteFile(linker, []byte{0}, 0o755); err != nil {
		t.Fatal(err)
	}
	worker := filepath.Join(dir, "sqyre-vision")
	if err := os.WriteFile(worker, []byte{0}, 0o755); err != nil {
		t.Fatal(err)
	}
	t.Setenv("APPDIR", dir)

	cmd := workerCommand(nil, worker, "ping")
	if cmd.Path != linker {
		t.Fatalf("cmd.Path=%q want linker %q", cmd.Path, linker)
	}
	if len(cmd.Args) != 3 || cmd.Args[0] != linker || cmd.Args[1] != worker || cmd.Args[2] != "ping" {
		t.Fatalf("unexpected args: %v", cmd.Args)
	}
}
