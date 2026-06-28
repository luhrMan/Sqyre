package hookkeys

import "testing"

func TestChordAllPressed(t *testing.T) {
	r := sliceReader{names: []string{"ctrl", "right"}}
	if !ChordAllPressed(r, []string{"ctrl", "right"}) {
		t.Fatal("expected chord pressed")
	}
	if ChordAllPressed(r, []string{"ctrl", "left"}) {
		t.Fatal("expected chord not pressed")
	}
}

type sliceReader struct {
	names []string
}

func (r sliceReader) PressedKeyNames() []string { return r.names }
func (sliceReader) Close()                      {}
