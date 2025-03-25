package encoding

import (
	"Squire/internal/config"
	"Squire/internal/programs/actions"
	"fmt"
	"log"
	"reflect"

	"github.com/go-viper/mapstructure/v2"
)

type sViper struct {
	serializer
}

func (s *sViper) Encode(d any) error {
	config.ViperConfig.Set("programs", d)
	err := config.ViperConfig.WriteConfig()
	if err != nil {
		return fmt.Errorf("error marshalling tree: %v", err)
	}
	log.Println("Successfully encoded:", "config.yaml")
	return nil
}

func (s *sViper) Decode(filename string, d any) error {
	err := config.ViperConfig.Unmarshal(&d)
	if err != nil {
		return fmt.Errorf("error unmarhsalling yaml file: %v", err)
	}
	log.Println("File successfuly decoded:", &d)

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
