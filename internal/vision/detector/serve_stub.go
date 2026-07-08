//go:build !detector_onnx

package detector

import (
	"fmt"
	"io"
)

// RunServe is unavailable without detector_onnx.
func RunServe(r io.Reader, w io.Writer) error {
	return fmt.Errorf("serve requires detector_onnx build")
}
