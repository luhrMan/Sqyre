package detector

// TextEncoder maps natural-language class prompts to CLIP text feature vectors.
// YOLO-World expects L2-normalized float32 vectors of dimension TextFeatureDim (512 for ViT-B/32).
type TextEncoder interface {
	Encode(prompts []string) ([][]float32, error)
}

const TextFeatureDim = 512

// StubTextEncoder returns zero vectors so the ONNX pipeline can be exercised without CLIP.
type StubTextEncoder struct{}

func (StubTextEncoder) Encode(prompts []string) ([][]float32, error) {
	out := make([][]float32, len(prompts))
	for i := range prompts {
		out[i] = make([]float32, TextFeatureDim)
	}
	return out, nil
}

// flattenTextFeatures packs [numClasses][512] into [numClasses*512] row-major for ONNX tensor creation.
func flattenTextFeatures(feats [][]float32) []float32 {
	if len(feats) == 0 {
		return nil
	}
	dim := len(feats[0])
	out := make([]float32, len(feats)*dim)
	for i, row := range feats {
		copy(out[i*dim:(i+1)*dim], row)
	}
	return out
}
