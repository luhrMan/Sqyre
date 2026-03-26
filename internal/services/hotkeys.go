package services

import (
	"Sqyre/internal/hotkeytrigger"
	"Sqyre/internal/models"
	"log"
	"os"
	"slices"
	"strings"
	"sync"
	"time"

	hook "github.com/luhrMan/gohook"
)

// Hook lifecycle: StartHook() is started in init() (see cmd/sqyre/sqyre.go). It runs
// hook.Start() then blocks on hook.Process(s), so all Register callbacks are dispatched
// from that goroutine. When unregistering from inside a callback, call hook.Unregister
// from a new goroutine (e.g. go hook.Unregister(...)) to avoid modifying hook state
// while Process is iterating over handlers.

func FailsafeHotkey() {
	fs := []string{"esc", "ctrl", "shift"}
	hook.Register(hook.KeyDown, fs, func(e hook.Event) {
		log.Println("FAILSAFE INITIATED: EXITING PROGRAM...")
		LogMatProfile()
		os.Exit(0)
	})
}

func StartHook() {
	log.Println("hook started")
	s := hook.Start()
	<-hook.Process(s)
}

func ParseMacroHotkey(hk string) []string {
	if hk == "" {
		return []string{}
	}
	parts := strings.Split(hk, "+")

	for i, part := range parts {
		parts[i] = strings.TrimSpace(part)
	}
	return parts
}

func ReverseParseMacroHotkey(hk []string) string {
	var str string
	for i, k := range hk {
		if i == 0 {
			str = k
			continue
		}
		str = str + " + " + k
	}
	return str
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
		// KeyDown fires repeatedly from OS key-repeat while the chord stays down; latch until release.
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

// UnregisterHotkeyKeys removes a prior registration for the given key chord and trigger (e.g. before changing hotkey or mode).
func UnregisterHotkeyKeys(hk []string, trigger string) {
	if len(hk) == 0 {
		return
	}
	t := models.ParseHotkeyTrigger(trigger)
	unregisterHotkeyByTrigger(hk, t)
}

func unregisterHotkeyByTrigger(hk []string, t models.HotkeyTrigger) {
	if t == models.HotkeyTriggerRelease {
		// Release uses KeyDown only (see registerReleaseHotkey); no per-key KeyUp hooks.
		hook.Unregister(hook.KeyDown, hk)
		return
	}
	hook.Unregister(hook.KeyDown, hk)
}

// registerReleaseHotkey runs the macro once after a full chord KeyDown and then a full release
// of every chord key. We cannot use gohook KeyUp handlers per key: Process clears the global
// uppressed map when the first KeyUp handler runs, so later keys' KeyUp callbacks often never fire.
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
					go ExecuteMacroWithLogging(m)
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
		go ExecuteMacroWithLogging(m)

		go func() {
			hotkeytrigger.WaitWhileAllPressed(func() bool { return hook.AllKeysPressed(hk) }, 8*time.Millisecond)
			hotkeytrigger.PressClearLatch(&mu, &latched)
		}()
	}
}

