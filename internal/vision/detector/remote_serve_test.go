package detector

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestStartWorker_preloadsModels(t *testing.T) {
	worker := ResolveWorkerPath()
	if worker == "" {
		t.Skip("sqyre-vision not installed")
	}
	if ResolvedORTLibrary() == "" {
		t.Skip("libonnxruntime not installed")
	}
	models := filepath.Join(os.Getenv("HOME"), ".sqyre", "models", defaultYOLOModelStem+onnxModelExt)
	if _, err := os.Stat(models); err != nil {
		t.Skip("vision models not downloaded")
	}

	t.Cleanup(ResetWarmUpForTesting)

	start := time.Now()
	if err := StartWorker(worker); err != nil {
		t.Fatal(err)
	}
	first := time.Since(start)

	start = time.Now()
	if err := StartWorker(worker); err != nil {
		t.Fatal(err)
	}
	second := time.Since(start)

	if second > first/2 {
		t.Fatalf("second StartWorker took %v, expected much less than first %v", second, first)
	}
}
