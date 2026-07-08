//go:build detector_onnx

package detector

import (
	"fmt"
	"path/filepath"

	ort "github.com/yalue/onnxruntime_go"
)

func newORTSessionOptions(kind modelFileKind, modelsDir, stem string) (*ort.SessionOptions, error) {
	opts, err := ort.NewSessionOptions()
	if err != nil {
		return nil, fmt.Errorf("create session options: %w", err)
	}

	var level ort.GraphOptimizationLevel = ort.GraphOptimizationLevelDisableAll
	if kind == modelONNX {
		level = ort.GraphOptimizationLevelEnableExtended
	}
	if err := opts.SetGraphOptimizationLevel(level); err != nil {
		opts.Destroy()
		return nil, fmt.Errorf("set graph optimization level: %w", err)
	}

	if kind == modelONNX && modelsDir != "" && stem != "" {
		optPath := filepath.Join(modelsDir, stem+optimizedONNXExt)
		if err := opts.SetOptimizedModelFilePath(optPath); err != nil {
			opts.Destroy()
			return nil, fmt.Errorf("set optimized model path: %w", err)
		}
	}

	if kind == modelORT {
		for _, kv := range [][2]string{
			{"session.use_memory_mapped_ort_model", "1"},
			{"session.use_ort_model_bytes_for_initializers", "1"},
		} {
			if err := opts.AddSessionConfigEntry(kv[0], kv[1]); err != nil {
				opts.Destroy()
				return nil, fmt.Errorf("%s: %w", kv[0], err)
			}
		}
	}
	return opts, nil
}
