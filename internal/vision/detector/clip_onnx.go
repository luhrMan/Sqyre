//go:build detector_onnx

package detector

import (
	"Sqyre/internal/vision/clipdata"
	"fmt"
	"sync"

	ort "github.com/yalue/onnxruntime_go"
)

type clipTextEncoder struct {
	session       *ort.AdvancedSession
	inputIDs      *ort.Tensor[int64]
	attentionMask *ort.Tensor[int64]
	output        *ort.Tensor[float32]
	mu            sync.Mutex
}

func newCLIPTextEncoder(modelsDir, ortLibPath string) (TextEncoder, error) {
	path, kind := resolveModelFile(modelsDir, defaultCLIPModelStem)
	if path == "" {
		return nil, fmt.Errorf("clip text model not found in %s", modelsDir)
	}
	if ortLibPath == "" {
		return nil, fmt.Errorf("%s not set and libonnxruntime not found", envORTLibPath)
	}
	if err := ensureORTEnv(ortLibPath); err != nil {
		return nil, fmt.Errorf("init onnxruntime for clip: %w", err)
	}

	shape := ort.NewShape(1, clipdata.MaxTokens)
	inputIDs, err := ort.NewEmptyTensor[int64](shape)
	if err != nil {
		return nil, err
	}
	attentionMask, err := ort.NewEmptyTensor[int64](shape)
	if err != nil {
		inputIDs.Destroy()
		return nil, err
	}
	outShape := ort.NewShape(1, clipdata.FeatureDim)
	output, err := ort.NewEmptyTensor[float32](outShape)
	if err != nil {
		inputIDs.Destroy()
		attentionMask.Destroy()
		return nil, err
	}

	opts, err := newORTSessionOptions(kind, modelsDir, defaultCLIPModelStem)
	if err != nil {
		inputIDs.Destroy()
		attentionMask.Destroy()
		output.Destroy()
		return nil, err
	}
	defer opts.Destroy()

	session, err := ort.NewAdvancedSession(
		path,
		[]string{"input_ids", "attention_mask"},
		[]string{"text_embeds"},
		[]ort.ArbitraryTensor{inputIDs, attentionMask},
		[]ort.ArbitraryTensor{output},
		opts,
	)
	if err != nil {
		inputIDs.Destroy()
		attentionMask.Destroy()
		output.Destroy()
		return nil, fmt.Errorf("load clip text session: %w", err)
	}
	return &clipTextEncoder{
		session:       session,
		inputIDs:      inputIDs,
		attentionMask: attentionMask,
		output:        output,
	}, nil
}

func (e *clipTextEncoder) Encode(prompts []string) ([][]float32, error) {
	e.mu.Lock()
	defer e.mu.Unlock()

	out := make([][]float32, len(prompts))
	for i, prompt := range prompts {
		ids, mask, err := clipdata.EncodeTokenIDs(prompt)
		if err != nil {
			return nil, err
		}
		copy(e.inputIDs.GetData(), ids)
		copy(e.attentionMask.GetData(), mask)
		if err := e.session.Run(); err != nil {
			return nil, fmt.Errorf("clip encode %q: %w", prompt, err)
		}
		vec := make([]float32, clipdata.FeatureDim)
		copy(vec, e.output.GetData())
		l2Normalize(vec)
		out[i] = vec
	}
	return out, nil
}

func l2Normalize(v []float32) {
	var sum float32
	for _, x := range v {
		sum += x * x
	}
	if sum <= 0 {
		return
	}
	inv := 1 / float32(sqrt32(sum))
	for i := range v {
		v[i] *= inv
	}
}

func sqrt32(x float32) float32 {
	if x <= 0 {
		return 0
	}
	z := x
	for i := 0; i < 6; i++ {
		z -= (z*z - x) / (2 * z)
	}
	return z
}
