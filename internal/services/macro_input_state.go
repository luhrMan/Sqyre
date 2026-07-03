package services

import "slices"

import "strings"

var modifierKeyNames = []string{
	"ctrl", "shift", "alt", "win", "cmd", "super",
}

var (
	macroHeldKeys = make(map[string]struct{})
)

func normalizeInputKey(key string) string {
	return strings.ToLower(strings.TrimSpace(key))
}

func resetMacroHeldKeys() {
	macroHeldKeys = make(map[string]struct{})
}

func noteMacroKeyDown(key string) {
	if !IsMacroRunning() {
		return
	}
	name := normalizeInputKey(key)
	if name == "" {
		return
	}
	macroHeldKeys[name] = struct{}{}
}

func noteMacroKeyUp(key string) {
	if !IsMacroRunning() {
		return
	}
	name := normalizeInputKey(key)
	if name == "" {
		return
	}
	delete(macroHeldKeys, name)
}

func releaseHeldMacroKeys() {
	backend := getAutomationBackend()
	var mods, triggers []string
	for k := range macroHeldKeys {
		if isModifierKey(k) {
			mods = append(mods, k)
		} else {
			triggers = append(triggers, k)
		}
	}
	for _, k := range triggers {
		_ = backend.KeyUp(k)
	}
	for _, k := range mods {
		_ = backend.KeyUp(k)
	}
	for _, k := range modifierKeyNames {
		_ = backend.KeyUp(k)
	}
}

func isModifierKey(key string) bool {
	return slices.Contains(modifierKeyNames, key)
}

// ReleaseAllMacroInputs sends key-up for held keys and common modifiers, and
// button-up for left, right, and center so no input remains physically held
// after a macro run ends.
func ReleaseAllMacroInputs() {
	releaseHeldMacroKeys()
	resetMacroHeldKeys()
	ReleaseAllMouseButtons()
}
