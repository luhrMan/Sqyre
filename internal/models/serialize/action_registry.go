package serialize

import (
	"Sqyre/internal/models/actions"
	"fmt"
)

type actionDecoder func(rawMap map[string]any) (actions.ActionInterface, error)
type actionEncoder func(action actions.ActionInterface) (map[string]any, error)

var (
	actionDecoders = map[string]actionDecoder{}
	actionEncoders = map[string]actionEncoder{}
)

func registerActionCodec(typeName string, decode actionDecoder, encode actionEncoder) {
	actionDecoders[typeName] = decode
	actionEncoders[typeName] = encode
}

func decodeActionFromMap(rawMap map[string]any) (actions.ActionInterface, error) {
	typeName, _ := rawMap["type"].(string)
	decode, ok := actionDecoders[typeName]
	if !ok {
		return nil, fmt.Errorf("unknown action type %v", rawMap["type"])
	}
	return decode(rawMap)
}

func encodeActionToMap(action actions.ActionInterface) (map[string]any, error) {
	if action == nil {
		return nil, fmt.Errorf("action is nil")
	}
	encode, ok := actionEncoders[action.GetType()]
	if !ok {
		return nil, fmt.Errorf("unknown action type: %T", action)
	}
	return encode(action)
}

func attachDecodedAction(s *serializer, action actions.ActionInterface, rawMap map[string]any, parent actions.AdvancedActionInterface) error {
	if uid := stringFromMap(rawMap, "uid"); uid != "" {
		actions.RestoreUID(action, uid)
	}
	action.SetParent(parent)
	if advAction, ok := action.(actions.AdvancedActionInterface); ok {
		if subActionsRaw, ok := rawMap["subactions"].([]any); ok {
			for i, subActionRaw := range subActionsRaw {
				subMap, ok := subActionRaw.(map[string]any)
				if !ok {
					return fmt.Errorf("subactions[%d]: expected mapping, got %T", i, subActionRaw)
				}
				subAction, err := s.CreateActionFromMap(subMap, advAction)
				if err != nil {
					return fmt.Errorf("subactions[%d]: %w", i, err)
				}
				advAction.AddSubAction(subAction)
			}
		}
	}
	return nil
}
