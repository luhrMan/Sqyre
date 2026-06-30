package serialize

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestActionToMap_breakContinue_roundTrip(t *testing.T) {
	for _, orig := range []actions.ActionInterface{actions.NewBreak(), actions.NewContinue()} {
		m, err := ActionToMap(orig)
		if err != nil {
			t.Fatalf("ActionToMap(%s): %v", orig.GetType(), err)
		}
		if m["type"] != orig.GetType() {
			t.Fatalf("type = %v, want %s", m["type"], orig.GetType())
		}
		back, err := ViperSerializer.CreateActionFromMap(m, nil)
		if err != nil {
			t.Fatalf("CreateActionFromMap(%s): %v", orig.GetType(), err)
		}
		if back.GetType() != orig.GetType() {
			t.Fatalf("round-trip type = %s, want %s", back.GetType(), orig.GetType())
		}
	}
}
