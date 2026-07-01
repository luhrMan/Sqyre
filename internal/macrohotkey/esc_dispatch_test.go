//go:build !nohook

package macrohotkey

import (
	"sync/atomic"
	"testing"
)

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
	for _, h := range handlers {
		h.fn()
	}
}
