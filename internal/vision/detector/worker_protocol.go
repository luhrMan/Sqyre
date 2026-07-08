package detector

// WorkerRequest is the JSON body sent to sqyre-vision on stdin for the detect subcommand.
type WorkerRequest struct {
	Prompts             []string `json:"prompts"`
	ImagePath           string   `json:"image_path"`
	ConfidenceThreshold float32  `json:"confidence_threshold"`
	IoUThreshold        float32  `json:"iou_threshold"`
	MaxMatches          int      `json:"max_matches"`
	InputSize           int      `json:"input_size"`
}

// WorkerBounds is a serializable rectangle.
type WorkerBounds struct {
	MinX int `json:"min_x"`
	MinY int `json:"min_y"`
	MaxX int `json:"max_x"`
	MaxY int `json:"max_y"`
}

// WorkerDetection is one match in the worker response.
type WorkerDetection struct {
	Label      string       `json:"label"`
	Confidence float32      `json:"confidence"`
	Bounds     WorkerBounds `json:"bounds"`
}

// WorkerResponse is written to stdout by sqyre-vision detect.
type WorkerResponse struct {
	Detections []WorkerDetection `json:"detections"`
	Error      string            `json:"error,omitempty"`
}

// WorkerReady is the first message on stdout when sqyre-vision serve finishes loading models.
type WorkerReady struct {
	Ready  bool  `json:"ready"`
	LoadMS int64 `json:"load_ms,omitempty"`
}
