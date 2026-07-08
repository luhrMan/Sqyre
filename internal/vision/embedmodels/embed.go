//go:build vision_embed

package embedmodels

import (
	"Sqyre/internal/config"
	"embed"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// Only source ONNX and ORT runtime are embedded — never .ort / .optimized.onnx caches.
//
//go:embed bundled/yolov8s-worldv2.onnx bundled/clip-text-vit-b32.onnx bundled/libonnxruntime.so
var bundled embed.FS

const (
	embeddedYOLO = "bundled/yolov8s-worldv2.onnx"
	embeddedCLIP = "bundled/clip-text-vit-b32.onnx"
	embeddedORT  = "bundled/libonnxruntime.so"
)

var (
	extractOnce sync.Once
	modelsDir   string
	extractErr  error
	ortLibPath  string
)

// Enabled reports whether this build embeds vision ONNX models.
func Enabled() bool { return true }

// EnsureModelsDir extracts embedded ONNX weights to ~/.sqyre/models on first use.
func EnsureModelsDir() (string, error) {
	extractOnce.Do(extractAll)
	if extractErr != nil {
		return "", extractErr
	}
	return modelsDir, nil
}

// EnsureORTLibrary extracts libonnxruntime.so to ~/.sqyre/lib when bundled.
func EnsureORTLibrary() (string, error) {
	extractOnce.Do(extractAll)
	if extractErr != nil {
		return "", extractErr
	}
	if ortLibPath != "" {
		return ortLibPath, nil
	}
	return "", fmt.Errorf("embedded onnxruntime library missing from vision build")
}

func extractAll() {
	modelsDir = config.GetModelsPath()
	libDir := filepath.Join(config.GetSqyreDir(), "lib")
	if err := os.MkdirAll(modelsDir, 0o755); err != nil {
		extractErr = err
		return
	}
	if err := os.MkdirAll(libDir, 0o755); err != nil {
		extractErr = err
		return
	}

	if err := extractBundledFile(embeddedYOLO, filepath.Join(modelsDir, "yolov8s-worldv2.onnx")); err != nil {
		extractErr = err
		return
	}
	if err := extractBundledFile(embeddedCLIP, filepath.Join(modelsDir, "clip-text-vit-b32.onnx")); err != nil {
		extractErr = err
		return
	}
	ortDst := filepath.Join(libDir, "libonnxruntime.so")
	if err := extractBundledFile(embeddedORT, ortDst); err != nil {
		extractErr = err
		return
	}
	ortLibPath = ortDst
}

func extractBundledFile(embedPath, dst string) error {
	data, err := bundled.ReadFile(embedPath)
	if err != nil {
		return fmt.Errorf("read %s: %w", embedPath, err)
	}
	return writeIfChanged(dst, data)
}

func writeIfChanged(path string, data []byte) error {
	if existing, err := os.ReadFile(path); err == nil && len(existing) == len(data) {
		same := true
		for i := range data {
			if existing[i] != data[i] {
				same = false
				break
			}
		}
		if same {
			return nil
		}
	}
	return os.WriteFile(path, data, 0o644)
}
