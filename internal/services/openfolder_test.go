package services

import (
	"os"
	"path/filepath"
	"testing"
)

func TestMoveDir_MovesTreeAndRemovesSource(t *testing.T) {
	root := t.TempDir()
	src := filepath.Join(root, "old", ".sqyre")
	dst := filepath.Join(root, "new", ".sqyre")

	nested := filepath.Join(src, "images", "icons")
	if err := os.MkdirAll(nested, 0755); err != nil {
		t.Fatalf("setup: %v", err)
	}
	if err := os.WriteFile(filepath.Join(nested, "a.png"), []byte("data"), 0644); err != nil {
		t.Fatalf("setup: %v", err)
	}

	if err := MoveDir(src, dst); err != nil {
		t.Fatalf("MoveDir: %v", err)
	}

	if _, err := os.Stat(src); !os.IsNotExist(err) {
		t.Errorf("source still exists after move: %v", err)
	}
	moved := filepath.Join(dst, "images", "icons", "a.png")
	got, err := os.ReadFile(moved)
	if err != nil {
		t.Fatalf("read moved file: %v", err)
	}
	if string(got) != "data" {
		t.Errorf("moved file contents = %q, want %q", got, "data")
	}
}

func TestMoveDir_RefusesExistingDestination(t *testing.T) {
	root := t.TempDir()
	src := filepath.Join(root, "src")
	dst := filepath.Join(root, "dst")
	for _, d := range []string{src, dst} {
		if err := os.MkdirAll(d, 0755); err != nil {
			t.Fatalf("setup: %v", err)
		}
	}

	if err := MoveDir(src, dst); err == nil {
		t.Fatal("expected error when destination exists, got nil")
	}
	if _, err := os.Stat(src); err != nil {
		t.Errorf("source should be untouched after refusal: %v", err)
	}
}

func TestMoveDir_SamePathIsNoop(t *testing.T) {
	dir := t.TempDir()
	if err := MoveDir(dir, dir); err != nil {
		t.Fatalf("MoveDir same path: %v", err)
	}
	if _, err := os.Stat(dir); err != nil {
		t.Errorf("dir should still exist: %v", err)
	}
}
