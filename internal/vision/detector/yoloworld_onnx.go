//go:build detector_onnx

package detector

import (
	"Sqyre/internal/config"
	"context"
	"fmt"
	"image"
	"log"
	"os"
	"sync"

	ort "github.com/yalue/onnxruntime_go"
)

// YOLOWorldDetector runs YOLOv8-World v2 ONNX with runtime text features.
type YOLOWorldDetector struct {
	session     *ort.AdvancedSession
	imageInput  *ort.Tensor[float32]
	textInput   *ort.Tensor[float32]
	output      *ort.Tensor[float32]
	textEncoder TextEncoder
	inputSize   int
	mu          sync.Mutex
}

func onnxDetectorAvailable() bool {
	_, ok := tryNewONNXDetector()
	return ok
}

func tryNewONNXDetector() (Detector, bool) {
	if path, _ := yoloModelPath(); path == "" {
		return nil, false
	}
	if ortLibraryPath() == "" {
		log.Printf("vision detector: %s not set and libonnxruntime not found", envORTLibPath)
		return nil, false
	}
	d, err := buildInProcessDetector()
	if err != nil {
		log.Printf("vision detector: %v", err)
		return nil, false
	}
	return d, true
}

func ortLibraryPath() string {
	return ResolvedORTLibrary()
}

func yoloModelPath() (string, modelFileKind) {
	return resolveModelFile(modelsDir(), defaultYOLOModelStem)
}

func modelsDir() string {
	if d := os.Getenv(envModelsDir); d != "" {
		return d
	}
	return config.GetModelsPath()
}

func newYOLOWorldDetector(modelPath string, kind modelFileKind, textEncoder TextEncoder) (*YOLOWorldDetector, error) {
	const inputSize = 640
	dir := modelsDir()
	imageShape := ort.NewShape(1, 3, int64(inputSize), int64(inputSize))
	imageInput, err := ort.NewEmptyTensor[float32](imageShape)
	if err != nil {
		return nil, fmt.Errorf("create image tensor: %w", err)
	}

	// txt_feats: [1, num_classes, 512] — num_classes is dynamic per call; allocate max 32 classes.
	const maxClasses = 32
	textShape := ort.NewShape(1, maxClasses, TextFeatureDim)
	textInput, err := ort.NewEmptyTensor[float32](textShape)
	if err != nil {
		imageInput.Destroy()
		return nil, fmt.Errorf("create text tensor: %w", err)
	}

	// output0: [1, 4+num_classes, num_anchors] — anchors depend on input size (8400 at 640).
	const numAnchors = 8400
	outputShape := ort.NewShape(1, int64(4+maxClasses), numAnchors)
	output, err := ort.NewEmptyTensor[float32](outputShape)
	if err != nil {
		imageInput.Destroy()
		textInput.Destroy()
		return nil, fmt.Errorf("create output tensor: %w", err)
	}

	opts, err := newORTSessionOptions(kind, dir, defaultYOLOModelStem)
	if err != nil {
		imageInput.Destroy()
		textInput.Destroy()
		output.Destroy()
		return nil, fmt.Errorf("session options: %w", err)
	}
	defer opts.Destroy()

	session, err := ort.NewAdvancedSession(
		modelPath,
		[]string{"images", "txt_feats"},
		[]string{"output0"},
		[]ort.ArbitraryTensor{imageInput, textInput},
		[]ort.ArbitraryTensor{output},
		opts,
	)
	if err != nil {
		imageInput.Destroy()
		textInput.Destroy()
		output.Destroy()
		return nil, fmt.Errorf("load yolo-world session: %w", err)
	}

	if textEncoder == nil {
		textEncoder = StubTextEncoder{}
	}

	return &YOLOWorldDetector{
		session:     session,
		imageInput:  imageInput,
		textInput:   textInput,
		output:      output,
		textEncoder: textEncoder,
		inputSize:   inputSize,
	}, nil
}

func (d *YOLOWorldDetector) Detect(ctx context.Context, frame image.Image, opts Options) ([]Detection, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	opts.applyDefaults()
	if len(opts.Prompts) == 0 {
		return nil, nil
	}
	if len(opts.Prompts) > 32 {
		return nil, fmt.Errorf("at most 32 prompts per detect call, got %d", len(opts.Prompts))
	}

	d.mu.Lock()
	defer d.mu.Unlock()

	lb := letterboxResize(frame, opts.InputSize)
	copy(d.imageInput.GetData(), lb.data)

	textFeats, err := d.textEncoder.Encode(opts.Prompts)
	if err != nil {
		return nil, fmt.Errorf("encode prompts: %w", err)
	}
	flat := flattenTextFeatures(textFeats)
	textData := d.textInput.GetData()
	copy(textData[:len(flat)], flat)

	if err := d.session.Run(); err != nil {
		return nil, fmt.Errorf("yolo-world inference: %w", err)
	}

	outData := d.output.GetData()
	numClasses := len(opts.Prompts)
	numAnchors := 8400
	boxes := decodeYOLOWorldOutput(outData, numClasses, numAnchors, opts.Prompts, lb, opts.ConfidenceThreshold)
	return nonMaxSuppression(boxes, opts.IoUThreshold, opts.MaxMatches), nil
}
