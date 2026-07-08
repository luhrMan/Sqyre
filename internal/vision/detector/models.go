package detector

import (
	"os"
	"path/filepath"
)

const (
	envORTLibPath = "SQUIRE_ORT_LIB"
	envModelsDir  = "SQUIRE_VISION_MODEL_DIR"
)

const (
	onnxModelExt       = ".onnx"
	ortModelExt        = ".ort"
	optimizedONNXExt   = ".optimized.onnx"
	defaultYOLOModelStem = "yolov8s-worldv2"
	defaultCLIPModelStem = "clip-text-vit-b32"
)

type modelFileKind int

const (
	modelONNX modelFileKind = iota
	modelOptimizedONNX
	modelORT
)

// ModelFileNames returns source ONNX filenames shipped or downloaded for vision.
func ModelFileNames() []string {
	return []string{
		defaultYOLOModelStem + onnxModelExt,
		defaultCLIPModelStem + onnxModelExt,
	}
}

// EnvModelsDir is the environment variable name for the models directory.
func EnvModelsDir() string { return envModelsDir }

// EnvORTLib is the environment variable name for libonnxruntime.
func EnvORTLib() string { return envORTLibPath }

func modelStems() []string {
	return []string{defaultYOLOModelStem, defaultCLIPModelStem}
}

func sourceONNXPath(dir, stem string) string {
	if dir == "" {
		return ""
	}
	return filepath.Join(dir, stem+onnxModelExt)
}

// resolveModelFile picks the best available model file in dir for stem.
// Preference: .ort cache, .optimized.onnx cache, source .onnx.
func resolveModelFile(dir, stem string) (path string, kind modelFileKind) {
	if dir == "" {
		return "", modelONNX
	}
	for _, entry := range []struct {
		ext  string
		kind modelFileKind
	}{
		{ortModelExt, modelORT},
		{optimizedONNXExt, modelOptimizedONNX},
		{onnxModelExt, modelONNX},
	} {
		candidate := filepath.Join(dir, stem+entry.ext)
		if st, err := os.Stat(candidate); err == nil && !st.IsDir() {
			return candidate, entry.kind
		}
	}
	return "", modelONNX
}

func cacheIsFresh(cachePath, sourcePath string) bool {
	cacheSt, err := os.Stat(cachePath)
	if err != nil {
		return false
	}
	srcSt, err := os.Stat(sourcePath)
	if err != nil {
		return false
	}
	return !cacheSt.ModTime().Before(srcSt.ModTime())
}
