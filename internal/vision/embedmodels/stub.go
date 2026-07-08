//go:build !vision_embed

package embedmodels

// Enabled reports whether this build embeds vision ONNX models.
func Enabled() bool { return false }

// EnsureModelsDir returns the external models directory (no extraction).
func EnsureModelsDir() (string, error) {
	return "", nil
}

// EnsureORTLibrary returns empty — use SQUIRE_ORT_LIB or system library.
func EnsureORTLibrary() (string, error) {
	return "", nil
}
