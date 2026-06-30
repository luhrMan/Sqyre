package serialize

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"fmt"
	"os"
	"path/filepath"
	"reflect"

	"github.com/go-viper/mapstructure/v2"
	"github.com/spf13/viper"
)

type sViper struct {
	serializer
}

var Viperizer *viper.Viper

func init() {
	Viperizer = viper.New()
}

func GetViper() *viper.Viper {
	return Viperizer
}

// func (s *sViper) Encode(d any) error {
// 	// s.encodePrograms(d.(map[string]program.Program))
// 	// s.encodeMacros()
// 	log.Println("Successfully encoded:", "yaml")
// 	return nil
// }

// LoadConfig ensures ~/.sqyre/db.yaml exists and loads it into YAMLConfig (single parse).
func LoadConfig() error {
	configPath := config.GetDbPath()

	if err := ensureConfigFile(configPath); err != nil {
		return fmt.Errorf("config setup: %w", err)
	}

	GetYAMLConfig().SetConfigFile(configPath)
	if err := GetYAMLConfig().ReadConfig(); err != nil {
		return fmt.Errorf("yaml db read: %w", err)
	}

	return nil
}

// ensureConfigFile creates ~/.sqyre and a minimal db.yaml if the file does not exist.
func ensureConfigFile(configPath string) error {
	if _, err := os.Stat(configPath); err == nil {
		return nil
	}
	dir := filepath.Dir(configPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create config dir: %w", err)
	}
	// Write minimal config so Viper and YAMLConfig can load
	body := []byte("macros: {}\nprograms: {}\n")
	if err := os.WriteFile(configPath, body, 0644); err != nil {
		return fmt.Errorf("write default config: %w", err)
	}
	return nil
}

func MacroDecodeHookFunc() mapstructure.DecodeHookFuncType {
	return func(
		f reflect.Type,
		t reflect.Type,
		data any,
	) (any, error) {
		if t == reflect.TypeOf(actions.Loop{}) {
			rawMap, ok := data.(map[string]any)
			if !ok {
				return data, fmt.Errorf("expected map[string]any, got %T", data)
			}

			_, exists := rawMap["type"]
			if !exists {
				return data, fmt.Errorf("missing 'type' field in map")
			}

			if rawMap["type"] != "loop" {
				return data, fmt.Errorf("missing 'loop' field in map")
			}

			data, err := ViperSerializer.CreateActionFromMap(rawMap, nil)
			if err != nil {
				return data, err
			}
			return data, nil
		}
		if t == reflect.TypeOf((*actions.ActionInterface)(nil)).Elem() {
			return nil, nil
		}

		return data, nil
	}
}

type ISerializer interface {
	Encode(string, any) error
	Decode(string) (map[string]any, error)
	CreateActionFromMap(map[string]any, actions.AdvancedActionInterface) (actions.ActionInterface, error)
}

type serializer struct {
	iSerializer ISerializer
}

var (
	ViperSerializer = sViper{}
	Serializer      = serializer{}.iSerializer
)

func (s *serializer) CreateActionFromMap(rawMap map[string]any, parent actions.AdvancedActionInterface) (actions.ActionInterface, error) {
	action, err := decodeActionFromMap(rawMap)
	if err != nil {
		return nil, err
	}
	if err := attachDecodedAction(s, action, rawMap, parent); err != nil {
		return nil, err
	}
	return action, nil
}

// targetsFromMap converts rawMap["targets"] to []string whether it is []string (from ActionToMap) or []any (from YAML).
func targetsFromMap(v any) []string {
	if v == nil {
		return nil
	}
	if ss, ok := v.([]string); ok {
		return ss
	}
	if slice, ok := v.([]any); ok {
		out := make([]string, 0, len(slice))
		for _, t := range slice {
			if s, ok := t.(string); ok {
				out = append(out, s)
			}
		}
		return out
	}
	return nil
}

func parseCoordinateRef(v any) actions.CoordinateRef {
	if v == nil {
		return ""
	}
	switch val := v.(type) {
	case string:
		return actions.CoordinateRef(val)
	case map[string]any:
		name := stringFromMap(val, "name")
		if name == "" {
			return ""
		}
		// Legacy embedded format: keep only the name as a lookup key.
		return actions.CoordinateRef(name)
	default:
		return ""
	}
}

// anySlice normalizes YAML/JSON slice values to []any.
func anySlice(v any) ([]any, error) {
	switch t := v.(type) {
	case []any:
		return t, nil
	case []map[string]any:
		out := make([]any, len(t))
		for i, m := range t {
			out[i] = m
		}
		return out, nil
	default:
		return nil, fmt.Errorf("expected list, got %T", v)
	}
}

// operandFromMap returns a conditional operand as int (literal) or string
// (literal or variable reference), defaulting to "" when missing.
func operandFromMap(rawMap map[string]any, key string) any {
	v, ok := rawMap[key]
	if !ok || v == nil {
		return ""
	}
	switch val := v.(type) {
	case int:
		return val
	case int64:
		return int(val)
	case float64:
		return int(val)
	case string:
		return val
	default:
		return fmt.Sprintf("%v", val)
	}
}

// rowBoundFromMap returns a For Each Row StartRow/EndRow value as int (literal)
// or string (variable reference), or nil when the key is absent.
func rowBoundFromMap(rawMap map[string]any, key string) any {
	v, ok := rawMap[key]
	if !ok || v == nil {
		return nil
	}
	switch val := v.(type) {
	case int:
		return val
	case int64:
		return int(val)
	case float64:
		return int(val)
	case string:
		return val
	default:
		return fmt.Sprintf("%v", val)
	}
}

func stringFromMap(m map[string]any, key string) string {
	if v, ok := m[key]; ok && v != nil {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}

func intFromMap(v any) int {
	if v == nil {
		return 0
	}
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

func floatFromMap(v any) float64 {
	if v == nil {
		return 0
	}
	switch n := v.(type) {
	case float64:
		return n
	case float32:
		return float64(n)
	case int:
		return float64(n)
	case int64:
		return float64(n)
	default:
		return 0
	}
}

// func calibrationTargetsFromMap(v any) []actions.CalibrationTarget {
// 	if v == nil {
// 		return nil
// 	}
// 	slice, ok := v.([]any)
// 	if !ok {
// 		return nil
// 	}
// 	out := make([]actions.CalibrationTarget, 0, len(slice))
// 	for _, e := range slice {
// 		m, ok := e.(map[string]any)
// 		if !ok {
// 			continue
// 		}
// 		out = append(out, actions.CalibrationTarget{
// 			OutputName: stringFromMap(m, "outputname"),
// 			OutputType: stringFromMap(m, "outputtype"),
// 			Target:     stringFromMap(m, "target"),
// 		})
// 	}
// 	return out
// }

