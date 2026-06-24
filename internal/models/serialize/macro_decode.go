package serialize

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"fmt"

	"gopkg.in/yaml.v3"
)

// DecodeMacroFromMap builds a Macro from a YAML map value without a viper round-trip.
func DecodeMacroFromMap(data any) (*models.Macro, error) {
	rawMap, ok := data.(map[string]any)
	if !ok {
		return nil, fmt.Errorf("expected map[string]any, got %T", data)
	}

	macro := models.NewMacro("", 0, nil)
	if name, ok := rawMap["name"].(string); ok {
		macro.Name = name
	}
	if gd := rawMap["globaldelay"]; gd != nil {
		macro.GlobalDelay = intFromAny(gd)
	}
	macro.Hotkey = stringSliceFromAny(rawMap["hotkey"])
	if ht, ok := rawMap["hotkey_trigger"].(string); ok {
		macro.HotkeyTrigger = ht
	}
	if v, ok := rawMap["variables"]; ok && v != nil {
		b, err := yaml.Marshal(v)
		if err != nil {
			return nil, fmt.Errorf("macro %q variables: %w", macro.Name, err)
		}
		if err := yaml.Unmarshal(b, &macro.VariableDecls); err != nil {
			return nil, fmt.Errorf("macro %q variables: %w", macro.Name, err)
		}
	}

	rootRaw, ok := rawMap["root"].(map[string]any)
	if !ok {
		return nil, fmt.Errorf("macro %q: missing or invalid root", macro.Name)
	}
	action, err := ViperSerializer.CreateActionFromMap(rootRaw, nil)
	if err != nil {
		return nil, fmt.Errorf("macro %q root: %w", macro.Name, err)
	}
	loop, ok := action.(*actions.Loop)
	if !ok {
		return nil, fmt.Errorf("macro %q: root must be a loop", macro.Name)
	}
	macro.Root = loop

	macro.InitRuntimeVariables()
	return macro, nil
}

func intFromAny(v any) int {
	switch n := v.(type) {
	case int:
		return n
	case int64:
		return int(n)
	case float64:
		return int(n)
	default:
		return 0
	}
}

func stringSliceFromAny(v any) []string {
	if v == nil {
		return nil
	}
	if ss, ok := v.([]string); ok {
		return ss
	}
	slice, ok := v.([]any)
	if !ok {
		return nil
	}
	out := make([]string, 0, len(slice))
	for _, e := range slice {
		if s, ok := e.(string); ok {
			out = append(out, s)
		}
	}
	return out
}
