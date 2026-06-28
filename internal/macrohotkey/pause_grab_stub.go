//go:build !linux || nohook

package macrohotkey

func grabContinueChord(_ []string, _ bool) (func(), error) {
	return func() {}, nil
}
