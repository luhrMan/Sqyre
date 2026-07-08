//go:build !nohook

package macrohotkey

import (
	"sync"

	hook "github.com/luhrMan/gohook"
)

var (
	enterDispatchMu     sync.Mutex
	enterHandlers       []enterHandler
	nextEnterHandlerID  int
	globalEnterHookOnce bool
)

type enterHandler struct {
	id int
	fn func()
}

// RegisterEnterHandler adds a callback for lone Enter key presses. When multiple
// handlers are registered (e.g. stacked popups), only the most recently registered
// handler runs. The returned function removes only this handler. gohook Unregister
// matches by key chord and removes an arbitrary first match, so enter handlers must
// not use hook.Unregister.
func RegisterEnterHandler(fn func()) (unregister func()) {
	if fn == nil {
		return func() {}
	}
	enterDispatchMu.Lock()
	defer enterDispatchMu.Unlock()
	id := nextEnterHandlerID
	nextEnterHandlerID++
	enterHandlers = append(enterHandlers, enterHandler{id: id, fn: fn})
	ensureGlobalEnterHookLocked()
	return func() { unregisterEnterHandler(id) }
}

func unregisterEnterHandler(id int) {
	enterDispatchMu.Lock()
	defer enterDispatchMu.Unlock()
	for i, h := range enterHandlers {
		if h.id == id {
			enterHandlers = append(enterHandlers[:i], enterHandlers[i+1:]...)
			return
		}
	}
}

func ensureGlobalEnterHookLocked() {
	if globalEnterHookOnce {
		return
	}
	globalEnterHookOnce = true
	hook.Register(hook.KeyDown, []string{"enter"}, func(hook.Event) {
		enterDispatchMu.Lock()
		handlers := append([]enterHandler(nil), enterHandlers...)
		enterDispatchMu.Unlock()
		if len(handlers) > 0 {
			handlers[len(handlers)-1].fn()
		}
	})
}
