//go:build detector_onnx

package detector

import "sync"

type lazyTextEncoder struct {
	load func() (TextEncoder, error)
	once sync.Once
	enc  TextEncoder
	err  error
}

func newLazyTextEncoder(load func() (TextEncoder, error)) *lazyTextEncoder {
	return &lazyTextEncoder{load: load}
}

// warm starts loading in the background so the first detect does not wait.
func (l *lazyTextEncoder) warm() {
	go func() {
		l.once.Do(func() {
			l.enc, l.err = l.load()
			if l.err != nil {
				l.enc = StubTextEncoder{}
			}
		})
	}()
}

func (l *lazyTextEncoder) Encode(prompts []string) ([][]float32, error) {
	l.once.Do(func() {
		l.enc, l.err = l.load()
		if l.err != nil {
			l.enc = StubTextEncoder{}
		}
	})
	if l.err != nil {
		return nil, l.err
	}
	return l.enc.Encode(prompts)
}
