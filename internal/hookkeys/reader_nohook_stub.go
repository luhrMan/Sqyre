//go:build nohook

package hookkeys

// NewReader returns a headless stub for builds with -tags=nohook (no X11/gohook).
func NewReader() (Reader, error) {
	return noopReader{}, nil
}

type noopReader struct{}

func (noopReader) PressedKeyNames() []string { return nil }
func (noopReader) Close()                    {}
