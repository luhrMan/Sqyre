package services

import "strings"

var modifierKeyNames = map[string]struct{}{
	"ctrl": {}, "shift": {}, "alt": {}, "win": {}, "cmd": {}, "super": {},
}

// SuppressContinueChord releases every key in a continue chord so a non-pass-through
// continue does not leave modifiers held or deliver the trigger key to the target app.
func SuppressContinueChord(keys []string) {
	if len(keys) == 0 {
		return
	}
	backend := getAutomationBackend()
	var mods, triggers []string
	for _, k := range keys {
		name := strings.ToLower(strings.TrimSpace(k))
		if name == "" {
			continue
		}
		if _, isMod := modifierKeyNames[name]; isMod {
			mods = append(mods, name)
		} else {
			triggers = append(triggers, name)
		}
	}
	for _, k := range triggers {
		_ = backend.KeyUp(k)
	}
	for _, k := range mods {
		_ = backend.KeyUp(k)
	}
}
