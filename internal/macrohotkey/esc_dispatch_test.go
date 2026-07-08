//go:build !nohook

package macrohotkey

import (
	"sync/atomic"
	"testing"
)

func TestRegisterEscapeHandler_OnlyTopmostDispatched(t *testing.T) {
	var first, second atomic.Int32
	RegisterEscapeHandler(func() { first.Add(1) })
	RegisterEscapeHandler(func() { second.Add(1) })

	dispatchEscapeHandlersForTest()

	if first.Load() != 0 {
		t.Fatalf("first handler calls = %d, want 0", first.Load())
	}
	if second.Load() != 1 {
		t.Fatalf("second handler calls = %d, want 1", second.Load())
	}
}

func TestRegisterEscapeHandler_UnregisterOnlySelf(t *testing.T) {
	var permanent, temporary atomic.Int32
	unregPermanent := RegisterEscapeHandler(func() { permanent.Add(1) })
	unregTemporary := RegisterEscapeHandler(func() { temporary.Add(1) })

	unregTemporary()
	dispatchEscapeHandlersForTest()

	if permanent.Load() != 1 {
		t.Fatalf("permanent handler calls = %d, want 1", permanent.Load())
	}
	if temporary.Load() != 0 {
		t.Fatalf("temporary handler calls = %d, want 0", temporary.Load())
	}

	unregPermanent()
}

func dispatchEscapeHandlersForTest() {
	escDispatchMu.Lock()
	handlers := append([]escHandler(nil), escHandlers...)
	escDispatchMu.Unlock()
	if len(handlers) > 0 {
		handlers[len(handlers)-1].fn()
	}
}
