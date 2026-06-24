package services

import (
	"sync"
	"testing"
)

func resetHighlightState() {
	SetHighlightEnabled(false)
	SetHighlightCallback(nil)
}

func TestHighlightCursorDeliversEveryUpdate(t *testing.T) {
	resetHighlightState()
	t.Cleanup(resetHighlightState)

	SetHighlightEnabled(true)

	var mu sync.Mutex
	var events []HighlightEvent
	SetHighlightCallback(func(ev HighlightEvent) {
		mu.Lock()
		events = append(events, ev)
		mu.Unlock()
	})

	highlightCursor("macro", "a")
	highlightCursor("macro", "b")
	highlightCursor("macro", "c")

	mu.Lock()
	defer mu.Unlock()
	if len(events) != 3 {
		t.Fatalf("got %d events, want 3 immediate deliveries", len(events))
	}
	if events[2].UID != "c" {
		t.Fatalf("last uid = %q, want c", events[2].UID)
	}
}

func TestHighlightCursorClearIsImmediate(t *testing.T) {
	resetHighlightState()
	t.Cleanup(resetHighlightState)

	SetHighlightEnabled(true)

	var mu sync.Mutex
	var events []HighlightEvent
	SetHighlightCallback(func(ev HighlightEvent) {
		mu.Lock()
		events = append(events, ev)
		mu.Unlock()
	})

	highlightCursor("macro", "a")
	highlightCursor("macro", "b")
	highlightCursor("macro", "")

	mu.Lock()
	defer mu.Unlock()
	if len(events) < 2 {
		t.Fatalf("events = %+v, want at least start and clear", events)
	}
	last := events[len(events)-1]
	if last.UID != "" || last.Kind != HighlightSimple {
		t.Fatalf("last event = %+v, want immediate clear", last)
	}
}
