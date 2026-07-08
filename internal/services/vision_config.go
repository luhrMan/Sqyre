package services

import "Sqyre/internal/vision/detector"

// ApplyVisionDetectorConfig syncs Fyne preferences into the detector package.
func ApplyVisionDetectorConfig(workerPath, modelsDir string) {
	detector.SetRuntimeConfig(detector.RuntimeConfig{
		WorkerPath: workerPath,
		ModelsDir:  modelsDir,
	})
}

// VisionWorkerStatus returns semantic vision availability for the settings UI.
func VisionWorkerStatus() (workerPath, modelsDir string, ok bool, detail string) {
	return detector.WorkerStatus()
}
