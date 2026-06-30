//go:build !linux && !nohook

package hookkeys

import hook "github.com/luhrMan/gohook"

// NewReader returns a reader backed by gohook pressed-key state.
func NewReader() (Reader, error) {
	return hookReader{}, nil
}

type hookReader struct{}

func (hookReader) PressedKeyNames() []string { return hook.PressedKeyNames() }
func (hookReader) Close()                    {}
