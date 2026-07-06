//go:build nohook

package macrohotkey

// RegisterEnterHandler is a no-op when built with -tags=nohook.
func RegisterEnterHandler(_ func()) (unregister func()) {
	return func() {}
}
