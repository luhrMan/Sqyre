package serialize

import (
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
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
		targets := make([]string, 0)
		for _, t := range rawMap["targets"].([]any) {
			targets = append(targets, t.(string))
		}
		action = actions.NewImageSearch(rawMap["name"].(string), []actions.ActionInterface{}, targets, createSearchBox(rawMap["searcharea"].(map[string]any)), rawMap["rowsplit"].(int), rawMap["colsplit"].(int), float32(rawMap["tolerance"].(float64)))
	case "ocr":
		action = actions.NewOcr(rawMap["name"].(string), []actions.ActionInterface{}, rawMap["target"].(string), createSearchBox(rawMap["searcharea"].(map[string]any)))
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

func createSearchBox(rawMap map[string]any) coordinates.SearchArea {
	return coordinates.SearchArea{
		Name:    rawMap["name"].(string),
		LeftX:   rawMap["leftx"].(int),
		TopY:    rawMap["topy"].(int),
		RightX:  rawMap["rightx"].(int),
		BottomY: rawMap["bottomy"].(int),
	}
}

func createPoint(rawMap map[string]any) coordinates.Point {
	return coordinates.Point{
		Name: rawMap["name"].(string),
		X:    rawMap["x"].(int),
		Y:    rawMap["y"].(int),
	}
}
