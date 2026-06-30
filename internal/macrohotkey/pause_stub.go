//go:build nohook

package macrohotkey

import (
	"Sqyre/internal/services"
	"errors"
	"log"
	"strings"
)

func init() {
	services.SetMacroHotkeySuspendFuncs(func() {}, func() {})
	services.SetContinueKeyWaitFunc(func(opts services.ContinueWaitOptions) error {
		_ = opts
		log.Println("Pause: continue-key wait skipped (built with nohook)")
		return errors.New("pause: continue key requires gohook (not available in this build)")
	})
}

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

// ValidateContinueKey rejects empty chords.
func ValidateContinueKey(keys []string) error {
	if len(NormalizeContinueKey(keys)) == 0 {
		return errors.New("pause: continue key not set")
	}
	return nil
}

// FormatContinueKey returns a human-readable chord label.
func FormatContinueKey(keys []string) string {
	return ReverseParseMacroHotkey(NormalizeContinueKey(keys))
}

// ValidateContinueKeyForUI validates a continue chord for the action editor.
func ValidateContinueKeyForUI(keys []string) error {
	return ValidateContinueKey(keys)
}
