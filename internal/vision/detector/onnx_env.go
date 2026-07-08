//go:build detector_onnx

package detector

import (
	"sync"

	ort "github.com/yalue/onnxruntime_go"
)

var (
	ortInitOnce sync.Once
	ortInitErr  error
)

func ensureORTEnv(ortLibPath string) error {
	ortInitOnce.Do(func() {
		if ortLibPath != "" {
			ort.SetSharedLibraryPath(ortLibPath)
		}
		ortInitErr = ort.InitializeEnvironment()
	})
	return ortInitErr
}
