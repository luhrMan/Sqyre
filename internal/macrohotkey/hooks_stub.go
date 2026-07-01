//go:build nohook

package macrohotkey

import "Sqyre/internal/models"

func FailsafeHotkey()   {}
func MacroStopHotkey()  {}
func StartHook()        {}

func SuspendMacroHotkeys() {}
func ResumeMacroHotkeys()  {}

func RegisterMacroHotkey(_ *models.Macro)   {}
func UnregisterMacroHotkey(_ *models.Macro) {}

func UnregisterHotkeyKeys(_ []string, _ string) {}
