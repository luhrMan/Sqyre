//go:build !nohook

package macrohotkey

import (
	"sync/atomic"
	"testing"
)

func TestRegisterEnterHandler_OnlyTopmostDispatched(t *testing.T) {
	var first, second atomic.Int32
	RegisterEnterHandler(func() { first.Add(1) })
	RegisterEnterHandler(func() { second.Add(1) })

	dispatchEnterHandlersForTest()

	if first.Load() != 0 {
		t.Fatalf("first handler calls = %d, want 0", first.Load())
	}
	if second.Load() != 1 {
		t.Fatalf("second handler calls = %d, want 1", second.Load())
	}
}

func TestRegisterEnterHandler_UnregisterOnlySelf(t *testing.T) {
	var permanent, temporary atomic.Int32
	unregPermanent := RegisterEnterHandler(func() { permanent.Add(1) })
	unregTemporary := RegisterEnterHandler(func() { temporary.Add(1) })

	unregTemporary()
	dispatchEnterHandlersForTest()

	if permanent.Load() != 1 {
		t.Fatalf("permanent handler calls = %d, want 1", permanent.Load())
	}
	if temporary.Load() != 0 {
		t.Fatalf("temporary handler calls = %d, want 0", temporary.Load())
	}

	unregPermanent()
}

func dispatchEnterHandlersForTest() {
	enterDispatchMu.Lock()
	handlers := append([]enterHandler(nil), enterHandlers...)
	enterDispatchMu.Unlock()
	if len(handlers) > 0 {
		handlers[len(handlers)-1].fn()
	}
}
