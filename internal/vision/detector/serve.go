//go:build detector_onnx

package detector

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"time"
)

// RunServe handles a long-lived worker session: load models once, then detect per request.
func RunServe(r io.Reader, w io.Writer) error {
	PrepareWorkerEnv()
	bw := bufio.NewWriter(w)
	enc := json.NewEncoder(bw)

	start := time.Now()
	if err := PreloadModels(); err != nil {
		_ = enc.Encode(WorkerResponse{Error: err.Error()})
		_ = bw.Flush()
		return err
	}
	if err := enc.Encode(WorkerReady{
		Ready:  true,
		LoadMS: time.Since(start).Milliseconds(),
	}); err != nil {
		return err
	}
	if err := bw.Flush(); err != nil {
		return err
	}

	dec := json.NewDecoder(r)
	for {
		var req WorkerRequest
		if err := dec.Decode(&req); err != nil {
			if err == io.EOF {
				return nil
			}
			_ = enc.Encode(WorkerResponse{Error: fmt.Errorf("decode request: %w", err).Error()})
			_ = bw.Flush()
			continue
		}
		resp, err := ExecuteWorkerRequest(req)
		if err != nil {
			_ = enc.Encode(WorkerResponse{Error: err.Error()})
		} else {
			_ = enc.Encode(resp)
		}
		if err := bw.Flush(); err != nil {
			return err
		}
	}
}
