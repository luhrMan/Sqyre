//go:build !nohook

package macrohotkey

import (
	"Sqyre/internal/services"
	"errors"
	"fmt"
	"slices"
	"strings"
)

func init() {
	services.SetMacroHotkeySuspendFuncs(SuspendMacroHotkeys, ResumeMacroHotkeys)
	services.SetContinueKeyWaitFunc(waitForContinueKey)
}

var failsafeHotkey = []string{"esc", "ctrl", "shift"}

// NormalizeContinueKey trims and lowercases recorded key names.
func NormalizeContinueKey(keys []string) []string {
	out := make([]string, 0, len(keys))
	for _, k := range keys {
		k = strings.ToLower(strings.TrimSpace(k))
		if k == "" {
			continue
		}
		out = append(out, k)
	}
	return out
}

// ValidateContinueKey rejects empty chords and the global failsafe combination.
func ValidateContinueKey(keys []string) error {
	if len(keys) == 0 {
		return errors.New("pause: continue key not set")
	}
	if slices.Equal(keys, failsafeHotkey) {
		return errors.New("pause: continue key cannot match the failsafe hotkey (esc + ctrl + shift)")
	}
	return nil
}

// FormatContinueKey returns a human-readable chord label.
func FormatContinueKey(keys []string) string {
	return ReverseParseMacroHotkey(NormalizeContinueKey(keys))
}

// ValidateContinueKeyForUI is like ValidateContinueKey but returns fmt errors for dialogs.
func ValidateContinueKeyForUI(keys []string) error {
	if err := ValidateContinueKey(NormalizeContinueKey(keys)); err != nil {
		return fmt.Errorf("%w", err)
	}
	return nil
}
