//go:build detector_onnx

package detector

import (
	"fmt"
	"log"
	"time"
)

// PreloadModels loads vision sessions for serve mode. YOLO loads synchronously;
// CLIP continues in the background so the worker can accept requests sooner.
func PreloadModels() error {
	cachedInProcessMu.Lock()
	defer cachedInProcessMu.Unlock()
	if cachedInProcess != nil {
		return nil
	}

	start := time.Now()
	d, err := buildInProcessDetector()
	if err != nil {
		return err
	}
	cachedInProcess = d
	log.Printf("vision: yolo ready in %s (clip loading in background)", time.Since(start).Round(time.Millisecond))
	return nil
}

func buildInProcessDetector() (Detector, error) {
	ortPath := ortLibraryPath()
	if ortPath == "" {
		return nil, fmt.Errorf("libonnxruntime not found")
	}
	if err := ensureORTEnv(ortPath); err != nil {
		return nil, err
	}

	dir := modelsDir()
	modelPath, kind := yoloModelPath()
	if modelPath == "" {
		return nil, fmt.Errorf("yolo-world model not found in %s", dir)
	}

	clipLazy := newLazyTextEncoder(func() (TextEncoder, error) {
		t0 := time.Now()
		enc, err := newCLIPTextEncoder(dir, ortPath)
		if err != nil {
			log.Printf("vision detector: %v (detection quality will be poor without CLIP)", err)
			return nil, err
		}
		log.Printf("vision: clip ready in %s", time.Since(t0).Round(time.Millisecond))
		return enc, nil
	})
	clipLazy.warm()

	d, err := newYOLOWorldDetector(modelPath, kind, clipLazy)
	if err != nil {
		return nil, err
	}
	return d, nil
}
