//go:build !nohook

package macrohotkey

import (
	"Sqyre/internal/hotkeytrigger"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/services"
	"log"
	"os"
	"slices"
	"sync"
	"time"

	hook "github.com/luhrMan/gohook"
)

var (
	macroHotkeySuspendMu    sync.Mutex
	macroHotkeySuspendCount int
)

// Hook lifecycle: StartHook() is started from cmd/sqyre when SQYRE_NO_HOOK is unset. It runs
// hook.Start() then blocks on hook.Process(s), so all Register callbacks are dispatched
// from that goroutine. When unregistering from inside a callback, call hook.Unregister
// from a new goroutine (e.g. go hook.Unregister(...)) to avoid modifying hook state
// while Process is iterating over handlers.

func FailsafeHotkey() {
	fs := []string{"esc", "ctrl", "shift"}
	hook.Register(hook.KeyDown, fs, func(e hook.Event) {
		log.Println("FAILSAFE INITIATED: EXITING PROGRAM...")
		services.LogMatProfile()
		os.Exit(0)
	})
}

// MacroStopHotkey registers Escape to stop the currently running macro.
func MacroStopHotkey() {
	RegisterEscapeHandler(func() {
		if services.ShouldEscapeStopMacro() {
			log.Println("Escape: stopping macro execution")
			services.RequestMacroStop()
		}
	})
}

func StartHook() {
	log.Println("hook started")
	s := hook.Start()
	<-hook.Process(s)
}

// SuspendMacroHotkeys unregisters every macro hotkey. Nested calls count; each Suspend must be
// paired with ResumeMacroHotkeys so hooks are restored when the outermost suspend ends.
func SuspendMacroHotkeys() {
	macroHotkeySuspendMu.Lock()
	defer macroHotkeySuspendMu.Unlock()
	if macroHotkeySuspendCount == 0 {
		for _, m := range repositories.MacroRepo().GetAll() {
			UnregisterMacroHotkey(m)
		}
	}
	macroHotkeySuspendCount++
}

// ResumeMacroHotkeys re-registers all macro hotkeys from the repository after the matching
// SuspendMacroHotkeys (see suspend refcount).
func ResumeMacroHotkeys() {
	macroHotkeySuspendMu.Lock()
	defer macroHotkeySuspendMu.Unlock()
	if macroHotkeySuspendCount == 0 {
		return
	}
	macroHotkeySuspendCount--
	if macroHotkeySuspendCount == 0 {
		for _, m := range repositories.MacroRepo().GetAll() {
			RegisterMacroHotkey(m)
		}
	}
}

// RegisterMacroHotkey registers the macro's global hotkey using its Hotkey slice and HotkeyTrigger.
func RegisterMacroHotkey(m *models.Macro) {
	if m == nil {
		return
	}
	if slices.Equal(m.Hotkey, []string{}) {
		log.Println("do not register empty hotkeys!")
		return
	}
	t := models.ParseHotkeyTrigger(m.HotkeyTrigger)
	log.Printf("registering hotkey %v trigger=%s", m.Hotkey, t)
	switch t {
	case models.HotkeyTriggerRelease:
		registerReleaseHotkey(m)
	default:
		hook.Register(hook.KeyDown, m.Hotkey, macroHotkeyCallbackPress(m))
	}
}

// UnregisterMacroHotkey removes a registration matching the macro's hotkey keys and trigger mode.
func UnregisterMacroHotkey(m *models.Macro) {
	if m == nil || len(m.Hotkey) == 0 {
		return
	}
	t := models.ParseHotkeyTrigger(m.HotkeyTrigger)
	log.Printf("unregistering hotkey: %v trigger=%s", m.Hotkey, t)
	unregisterHotkeyByTrigger(m.Hotkey, t)
}

// UnregisterHotkeyKeys removes a prior registration for the given key chord and trigger.
func UnregisterHotkeyKeys(hk []string, trigger string) {
	if len(hk) == 0 {
		return
	}
	t := models.ParseHotkeyTrigger(trigger)
	unregisterHotkeyByTrigger(hk, t)
}

func unregisterHotkeyByTrigger(hk []string, t models.HotkeyTrigger) {
	if t == models.HotkeyTriggerRelease {
		hook.Unregister(hook.KeyDown, hk)
		return
	}
	hook.Unregister(hook.KeyDown, hk)
}

func registerReleaseHotkey(m *models.Macro) {
	hk := append([]string(nil), m.Hotkey...)
	var mu sync.Mutex
	var watching bool

	arm := func(hook.Event) {
		mu.Lock()
		if watching {
			mu.Unlock()
			return
		}
		watching = true
		mu.Unlock()

		go func() {
			defer func() {
				mu.Lock()
				watching = false
				mu.Unlock()
			}()
			hotkeytrigger.RunAfterChordThenFullRelease(
				func() bool { return hook.AllKeysPressed(hk) },
				func() bool { return hook.ChordFullyReleased(hk) },
				8*time.Millisecond,
				func() {
					log.Printf("hotkey %v (release), executing %v", hk, m.Name)
					go services.ExecuteMacroWithLogging(m)
				},
			)
		}()
	}

	hook.Register(hook.KeyDown, hk, arm)
}

func macroHotkeyCallbackPress(m *models.Macro) func(e hook.Event) {
	var mu sync.Mutex
	latched := false
	hk := append([]string(nil), m.Hotkey...)

	return func(e hook.Event) {
		if !hotkeytrigger.PressTryAcquireLatch(&mu, &latched) {
			return
		}

		log.Printf("hotkey %v (press), executing %v", m.Hotkey, m.Name)
		go services.ExecuteMacroWithLogging(m)

		go func() {
			hotkeytrigger.WaitWhileAllPressed(func() bool { return hook.AllKeysPressed(hk) }, 8*time.Millisecond)
			hotkeytrigger.PressClearLatch(&mu, &latched)
		}()
	}
}
