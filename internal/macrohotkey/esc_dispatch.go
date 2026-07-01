//go:build !nohook

package macrohotkey

import (
	"sync"

	hook "github.com/luhrMan/gohook"
)

var (
	escDispatchMu     sync.Mutex
	escHandlers       []escHandler
	nextEscHandlerID  int
	globalEscHookOnce bool
)

type escHandler struct {
	id int
	fn func()
}

// RegisterEscapeHandler adds a callback for lone Escape key presses. The returned
// function removes only this handler. gohook Unregister matches by key chord and
// removes an arbitrary first match, so esc handlers must not use hook.Unregister.
func RegisterEscapeHandler(fn func()) (unregister func()) {
	if fn == nil {
		return func() {}
	}
	escDispatchMu.Lock()
	defer escDispatchMu.Unlock()
	id := nextEscHandlerID
	nextEscHandlerID++
	escHandlers = append(escHandlers, escHandler{id: id, fn: fn})
	ensureGlobalEscHookLocked()
	return func() { unregisterEscapeHandler(id) }
}

func unregisterEscapeHandler(id int) {
	escDispatchMu.Lock()
	defer escDispatchMu.Unlock()
	for i, h := range escHandlers {
		if h.id == id {
			escHandlers = append(escHandlers[:i], escHandlers[i+1:]...)
			return
		}
	}
}

func ensureGlobalEscHookLocked() {
	if globalEscHookOnce {
		return
	}
	globalEscHookOnce = true
	hook.Register(hook.KeyDown, []string{"esc"}, func(hook.Event) {
		escDispatchMu.Lock()
		handlers := append([]escHandler(nil), escHandlers...)
		escDispatchMu.Unlock()
		for _, h := range handlers {
			h.fn()
		}
	})
}
