package services

// SuppressContinueChord releases every key in a continue chord so a non-pass-through
// continue does not leave modifiers held or deliver the trigger key to the target app.
func SuppressContinueChord(keys []string) {
	if len(keys) == 0 {
		return
	}
	backend := getAutomationBackend()
	var mods, triggers []string
	for _, k := range keys {
		name := normalizeInputKey(k)
		if name == "" {
			continue
		}
		if isModifierKey(name) {
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
