package detector

import (
	"bytes"
	"context"
	"fmt"
	"image"
	"image/png"
	"os"
)

// RemoteDetector runs inference in a separate sqyre-vision process.
type RemoteDetector struct {
	workerPath string
}

// NewRemoteDetector creates a detector that shells out to sqyre-vision.
func NewRemoteDetector(workerPath string) *RemoteDetector {
	return &RemoteDetector{workerPath: workerPath}
}

func (r *RemoteDetector) Detect(ctx context.Context, frame image.Image, opts Options) ([]Detection, error) {
	opts.applyDefaults()
	if len(opts.Prompts) == 0 {
		return nil, nil
	}

	tmp, err := os.CreateTemp("", "sqyre-vision-*.png")
	if err != nil {
		return nil, fmt.Errorf("create temp image: %w", err)
	}
	tmpPath := tmp.Name()
	defer os.Remove(tmpPath)

	if err := png.Encode(tmp, frame); err != nil {
		tmp.Close()
		return nil, fmt.Errorf("encode capture: %w", err)
	}
	if err := tmp.Close(); err != nil {
		return nil, err
	}

	req := WorkerRequest{
		Prompts:             opts.Prompts,
		ImagePath:           tmpPath,
		ConfidenceThreshold: opts.ConfidenceThreshold,
		IoUThreshold:        opts.IoUThreshold,
		MaxMatches:          opts.MaxMatches,
		InputSize:           opts.InputSize,
	}
	resp, err := getRemoteWorker(r.workerPath).detect(ctx, req)
	if err != nil {
		return nil, err
	}
	return workerDetectionsToLocal(resp.Detections), nil
}

func workerEnv() []string {
	env := os.Environ()
	if dir := resolvedModelsDir(); dir != "" {
		env = append(env, envModelsDir+"="+dir)
	}
	if ort := ResolvedORTLibrary(); ort != "" {
		env = append(env, envORTLibPath+"="+ort)
	}
	return env
}

func workerDetectionsToLocal(in []WorkerDetection) []Detection {
	out := make([]Detection, 0, len(in))
	for _, d := range in {
		out = append(out, Detection{
			Label:      d.Label,
			Confidence: d.Confidence,
			Bounds: image.Rect(
				d.Bounds.MinX, d.Bounds.MinY,
				d.Bounds.MaxX, d.Bounds.MaxY,
			),
		})
	}
	return out
}

// WorkerPing checks that sqyre-vision responds to the ping subcommand.
func WorkerPing(workerPath string) error {
	if workerPath == "" {
		return fmt.Errorf("vision worker not configured")
	}
	cmd := workerCommand(nil, workerPath, "ping")
	cmd.Env = workerEnv()
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("%w: %s", err, bytes.TrimSpace(out))
	}
	return nil
}

// WorkerStatus summarizes whether semantic vision can run.
func WorkerStatus() (workerPath string, modelsDir string, ok bool, detail string) {
	workerPath = ResolveWorkerPath()
	modelsDir = resolvedModelsDir()
	if workerPath == "" {
		return "", modelsDir, false, "sqyre-vision not found (install the vision bundle or set worker path in Settings)"
	}
	if err := WorkerPing(workerPath); err != nil {
		return workerPath, modelsDir, false, err.Error()
	}
	if ResolvedORTLibrary() == "" {
		return workerPath, modelsDir, false, "libonnxruntime not found (run make vision-models)"
	}
	yolo := sourceONNXPath(modelsDir, defaultYOLOModelStem)
	if _, err := os.Stat(yolo); err != nil {
		return workerPath, modelsDir, true, "worker ready; run make vision-models or use the embedded vision build (" + modelsDir + ")"
	}
	return workerPath, modelsDir, true, "ready"
}
