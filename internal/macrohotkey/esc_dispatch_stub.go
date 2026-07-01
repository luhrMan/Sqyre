//go:build nohook

package macrohotkey

// RegisterEscapeHandler is a no-op when built with -tags=nohook.
func RegisterEscapeHandler(_ func()) (unregister func()) {
	return func() {}
}
