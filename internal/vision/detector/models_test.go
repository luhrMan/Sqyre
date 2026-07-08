package detector

import (
	"os"
	"path/filepath"
	"testing"
)

func TestResolveModelFile(t *testing.T) {
	t.Helper()
	dir := t.TempDir()
	path, kind := resolveModelFile(dir, "missing")
	if path != "" || kind != modelONNX {
		t.Fatalf("resolveModelFile missing = (%q, %v), want empty ONNX", path, kind)
	}

	onnx := filepath.Join(dir, defaultYOLOModelStem+onnxModelExt)
	if err := os.WriteFile(onnx, []byte("onnx"), 0o644); err != nil {
		t.Fatal(err)
	}
	path, kind = resolveModelFile(dir, defaultYOLOModelStem)
	if path != onnx || kind != modelONNX {
		t.Fatalf("resolveModelFile onnx = (%q, %v), want (%q, modelONNX)", path, kind, onnx)
	}

	opt := filepath.Join(dir, defaultYOLOModelStem+optimizedONNXExt)
	if err := os.WriteFile(opt, []byte("opt"), 0o644); err != nil {
		t.Fatal(err)
	}
	path, kind = resolveModelFile(dir, defaultYOLOModelStem)
	if path != opt || kind != modelOptimizedONNX {
		t.Fatalf("resolveModelFile opt = (%q, %v), want (%q, modelOptimizedONNX)", path, kind, opt)
	}

	ortFile := filepath.Join(dir, defaultYOLOModelStem+ortModelExt)
	if err := os.WriteFile(ortFile, []byte("ort"), 0o644); err != nil {
		t.Fatal(err)
	}
	path, kind = resolveModelFile(dir, defaultYOLOModelStem)
	if path != ortFile || kind != modelORT {
		t.Fatalf("resolveModelFile ort = (%q, %v), want (%q, modelORT)", path, kind, ortFile)
	}
}

func TestCacheIsFresh(t *testing.T) {
	t.Helper()
	dir := t.TempDir()
	source := filepath.Join(dir, "model.onnx")
	cache := filepath.Join(dir, "model.ort")
	if err := os.WriteFile(source, []byte("src"), 0o644); err != nil {
		t.Fatal(err)
	}
	if cacheIsFresh(cache, source) {
		t.Fatal("missing cache should not be fresh")
	}
	if err := os.WriteFile(cache, []byte("cache"), 0o644); err != nil {
		t.Fatal(err)
	}
	if !cacheIsFresh(cache, source) {
		t.Fatal("newer cache should be fresh")
	}
}

func TestModelFileNames(t *testing.T) {
	t.Helper()
	names := ModelFileNames()
	if len(names) != 2 {
		t.Fatalf("ModelFileNames len = %d, want 2", len(names))
	}
	if names[0] != defaultYOLOModelStem+onnxModelExt {
		t.Fatalf("names[0] = %q", names[0])
	}
	if names[1] != defaultCLIPModelStem+onnxModelExt {
		t.Fatalf("names[1] = %q", names[1])
	}
}
