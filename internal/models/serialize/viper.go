package serialize

import (
	"Squire/internal/models/actions"
	"fmt"
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

func Decode() error {
	// v := viper.New()
	// ViperInit()

	GetViper().AddConfigPath("../../internal/config/")

	GetViper().SetConfigName("config")
	GetViper().SetConfigType("yaml")
	err := GetViper().ReadInConfig()
	if err != nil {
		return fmt.Errorf("viper error reading in file: %v", err)
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
	var action actions.ActionInterface
	switch rawMap["type"] {
	case "loop":
		action = actions.NewLoop(rawMap["count"].(int), rawMap["name"].(string), []actions.ActionInterface{})
	case "wait":
		action = actions.NewWait(rawMap["time"].(int))
	case "click":
		action = actions.NewClick(rawMap["button"].(bool))
	case "move":
		action = actions.NewMove(createPoint(rawMap["point"].(map[string]any)))
	case "key":
		action = actions.NewKey(rawMap["key"].(string), rawMap["state"].(bool))
	case "imagesearch":
		targets := targetsFromMap(rawMap["targets"])
		action = actions.NewImageSearch(rawMap["name"].(string), []actions.ActionInterface{}, targets, createSearchBox(rawMap["searcharea"].(map[string]any)), rawMap["rowsplit"].(int), rawMap["colsplit"].(int), float32(rawMap["tolerance"].(float64)))
		if is, ok := action.(*actions.ImageSearch); ok {
			if v, ok := rawMap["outputxvariable"].(string); ok {
				is.OutputXVariable = v
			}
			if v, ok := rawMap["outputyvariable"].(string); ok {
				is.OutputYVariable = v
			}
		}
	case "ocr":
		action = actions.NewOcr(rawMap["name"].(string), []actions.ActionInterface{}, rawMap["target"].(string), createSearchBox(rawMap["searcharea"].(map[string]any)))
	case "setvariable":
		action = actions.NewSetVariable(rawMap["variablename"].(string), rawMap["value"])
	case "calculate":
		action = actions.NewCalculate(rawMap["expression"].(string), rawMap["outputvar"].(string))
	case "datalist":
		isFile := false
		if ifVal, ok := rawMap["isfile"]; ok {
			isFile = ifVal.(bool)
		}
		action = actions.NewDataList(rawMap["source"].(string), rawMap["outputvar"].(string), isFile)
	case "savevariable":
		append := false
		if appendVal, ok := rawMap["append"]; ok {
			append = appendVal.(bool)
		}
		action = actions.NewSaveVariable(rawMap["variablename"].(string), rawMap["destination"].(string), append)
	}
	action.SetParent(parent)
	if advAction, ok := action.(actions.AdvancedActionInterface); ok {
		if subActionsRaw, ok := rawMap["subactions"].([]any); ok {
			for _, subActionRaw := range subActionsRaw {
				subAction, err := s.CreateActionFromMap(subActionRaw.(map[string]any), advAction)
				if err != nil {
					return nil, err
				}
				advAction.AddSubAction(subAction)
			}
		}
	}
	// log.Printf("Unmarshalled action %s", action)
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

func createSearchBox(rawMap map[string]any) actions.SearchArea {
	return actions.SearchArea{
		Name:    rawMap["name"].(string),
		LeftX:   valueAsIntOrString(rawMap["leftx"]),
		TopY:    valueAsIntOrString(rawMap["topy"]),
		RightX:  valueAsIntOrString(rawMap["rightx"]),
		BottomY: valueAsIntOrString(rawMap["bottomy"]),
	}
}

// valueAsIntOrString converts an interface{} to either int or string as appropriate for SearchArea fields.
func valueAsIntOrString(val interface{}) interface{} {
	switch v := val.(type) {
	case int:
		return v
	case float64:
		return int(v)
	case string:
		return v
	default:
		return 0
	}
}

// pointCoordFromMap returns x or y from raw map as interface{} (int or string) for actions.Point.
func pointCoordFromMap(rawMap map[string]any, key string) interface{} {
	v, ok := rawMap[key]
	if !ok {
		return 0
	}
	switch val := v.(type) {
	case int:
		return val
	case string:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

func createPoint(rawMap map[string]any) actions.Point {
	return actions.Point{
		Name: rawMap["name"].(string),
		X:    pointCoordFromMap(rawMap, "x"),
		Y:    pointCoordFromMap(rawMap, "y"),
	}
}
