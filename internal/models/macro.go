package models

import (
	"Sqyre/internal/models/actions"
	"sort"
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
	seen := make(map[string]struct{})

	builtins := []string{"StackMax", "Cols", "Rows", "ItemName", "ImagePixelWidth", "ImagePixelHeight"}
	for _, b := range builtins {
		seen[b] = struct{}{}
	}

	if m.Variables != nil {
		for _, name := range m.Variables.GetAll() {
			seen[name] = struct{}{}
		}
	}

	if m.Root != nil {
		collectVarsFromAction(m.Root, seen)
	}

	names := make([]string, 0, len(seen))
	for name := range seen {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

func collectVarsFromAction(a actions.ActionInterface, seen map[string]struct{}) {
	switch n := a.(type) {
	case *actions.SetVariable:
		if n.VariableName != "" {
			seen[n.VariableName] = struct{}{}
		}
	case *actions.Calculate:
		if n.OutputVar != "" {
			seen[n.OutputVar] = struct{}{}
		}
	case *actions.DataList:
		if n.OutputVar != "" {
			seen[n.OutputVar] = struct{}{}
		}
		if n.LengthVar != "" {
			seen[n.LengthVar] = struct{}{}
		}
	case *actions.Ocr:
		if n.OutputVariable != "" {
			seen[n.OutputVariable] = struct{}{}
		}
		if n.OutputXVariable != "" {
			seen[n.OutputXVariable] = struct{}{}
		}
		if n.OutputYVariable != "" {
			seen[n.OutputYVariable] = struct{}{}
		}
	case *actions.ImageSearch:
		if n.OutputXVariable != "" {
			seen[n.OutputXVariable] = struct{}{}
		}
		if n.OutputYVariable != "" {
			seen[n.OutputYVariable] = struct{}{}
		}
	}

	if adv, ok := a.(actions.AdvancedActionInterface); ok {
		for _, sub := range adv.GetSubActions() {
			collectVarsFromAction(sub, seen)
		}
	}
}
