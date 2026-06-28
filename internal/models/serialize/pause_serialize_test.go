package serialize

import (
	"Sqyre/internal/models/actions"
	"testing"
)

func TestActionToMap_Pause(t *testing.T) {
	p := actions.NewPause("wait here", []string{"ctrl", "f9"}, true)
	m, err := ActionToMap(p)
	if err != nil {
		t.Fatal(err)
	}
	if m["type"] != "pause" {
		t.Fatalf("type = %v", m["type"])
	}
	if m["message"] != "wait here" {
		t.Fatalf("message = %v", m["message"])
	}
	keys, ok := m["continuekey"].([]string)
	if !ok || len(keys) != 2 {
		t.Fatalf("continuekey = %v", m["continuekey"])
	}
	if m["passthrough"] != true {
		t.Fatalf("passthrough = %v", m["passthrough"])
	}

	round, err := ViperSerializer.CreateActionFromMap(m, nil)
	if err != nil {
		t.Fatal(err)
	}
	rp, ok := round.(*actions.Pause)
	if !ok {
		t.Fatalf("got %T", round)
	}
	if rp.Message != "wait here" || !rp.PassThrough || len(rp.ContinueKey) != 2 {
		t.Fatalf("round trip: %+v", rp)
	}
}
