package models

import (
	"Sqyre/internal/models/actions"
	"strings"
)

// HotkeyTrigger selects when a macro hotkey runs: chord complete on press or after full chord release.
type HotkeyTrigger string

const (
	HotkeyTriggerPress   HotkeyTrigger = "press"
	HotkeyTriggerRelease HotkeyTrigger = "release"
)

// ParseHotkeyTrigger normalizes persisted or UI values to a trigger. Unknown values default to press.
func ParseHotkeyTrigger(s string) HotkeyTrigger {
	switch strings.ToLower(strings.TrimSpace(s)) {
	case string(HotkeyTriggerRelease):
		return HotkeyTriggerRelease
	default:
		return HotkeyTriggerPress
	}
}

// UILabel returns the macro toolbar radio label for this trigger.
func (t HotkeyTrigger) UILabel() string {
	switch t {
	case HotkeyTriggerRelease:
		return "On release"
	default:
		return "On press"
	}
}

// HotkeyTriggerFromUILabel maps a radio option back to a persisted trigger value.
func HotkeyTriggerFromUILabel(s string) HotkeyTrigger {
	switch s {
	case "On release":
		return HotkeyTriggerRelease
	default:
		return HotkeyTriggerPress
	}
}

type Macro struct {
	Name           string         `mapstructure:"name"`
	Root           *actions.Loop  `mapstructure:"root"`
	GlobalDelay    int            `mapstructure:"globaldelay"`
	Hotkey         []string       `mapstructure:"hotkey"`
	HotkeyTrigger  string         `mapstructure:"hotkey_trigger"`
	Variables      *VariableStore `mapstructure:"variables"`
}

// GetKey returns the unique identifier for this Macro.
func (m *Macro) GetKey() string {
	return m.Name
}

// SetKey updates the unique identifier for this Macro.
func (m *Macro) SetKey(key string) {
	m.Name = key
}

// NewMacro creates a new Macro instance with the given parameters.
// The macro is initialized with an empty root loop.
func NewMacro(name string, delay int, hotkey []string) *Macro {
	return &Macro{
		Name:        name,
		Root:        actions.NewLoop(1, "root", []actions.ActionInterface{}),
		GlobalDelay: delay,
		Hotkey:      hotkey,
		Variables:   NewVariableStore(),
	}
}

// CollectDefinedVariables walks the macro's action tree and returns a sorted,
// deduplicated list of every variable name that actions define or output.
func (m *Macro) CollectDefinedVariables() []string {
	return CollectDefinedVariableNames(m)
}

// CollectVariableDefs returns variable definitions with source metadata.
func (m *Macro) CollectVariableDefs() []VariableDef {
	return CollectVariableDefs(m)
}

// SetInitialVariable sets a preset value stored on the macro (persisted in YAML).
func (m *Macro) SetInitialVariable(name, value string) {
	name = strings.TrimSpace(name)
	if name == "" {
		return
	}
	if m.Variables == nil {
		m.Variables = NewVariableStore()
	}
	if value == "" {
		m.Variables.Delete(name)
		return
	}
	m.Variables.Set(name, value)
}

// FindActionByUID returns the action with the given UID in this macro's tree.
func (m *Macro) FindActionByUID(uid string) actions.ActionInterface {
	if m == nil || m.Root == nil || uid == "" {
		return nil
	}
	return m.Root.GetAction(uid)
}
