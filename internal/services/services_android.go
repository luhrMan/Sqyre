//go:build android

package services

import (
	"Squire/internal/models"
	"strings"
)

// Event is a stub type for Android (desktop uses hook.Event).
type Event struct{}

func FailsafeHotkey() {}

func StartHook() {
	// No global hook on Android; block so the goroutine doesn't exit and cause issues.
	select {}
}

func ParseMacroHotkey(hk string) []string {
	if hk == "" {
		return nil
	}
	parts := strings.Split(hk, "+")
	for i := range parts {
		parts[i] = strings.TrimSpace(parts[i])
	}
	return parts
}

func ReverseParseMacroHotkey(hk []string) string {
	return strings.Join(hk, " + ")
}

func RegisterHotkey(hk []string, cb func(Event)) {}

func UnregisterHotkey(hk []string) {}

func MacroHotkeyCallback(m *models.Macro) func(Event) {
	return func(Event) {}
}

// Execute, ActiveWindowNames, RunFocusWindow: see executor_android.go

// Execute is implemented in executor_android.go.

type stubTessClient struct{}

func (stubTessClient) Close() {}

func GetTessClient() *stubTessClient { return nil }

func CloseTessClient() {}

// ActiveWindowNames and RunFocusWindow are implemented in executor_android.go (via android bridge).
