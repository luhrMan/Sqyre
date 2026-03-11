//go:build android

package services

import (
	"Squire/internal/models"
	"errors"
	"strings"

	"Squire/internal/models/actions"
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

func Execute(a actions.ActionInterface, macro ...*models.Macro) error {
	return errors.New("macro execution not supported on Android")
}

type stubTessClient struct{}

func (stubTessClient) Close() {}

func GetTessClient() *stubTessClient { return nil }

func CloseTessClient() {}

func ActiveWindowNames() ([]string, error) {
	return nil, errors.New("not supported on Android")
}

func RunFocusWindow(a *actions.FocusWindow) error {
	return errors.New("not supported on Android")
}
