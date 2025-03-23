package encoding

import (
	"Squire/internal/actions"
	"Squire/internal/data"
	"log"
)

type ISerializer interface {
	Encode(string, any) error
	Decode(string) (map[string]any, error)
	CreateActionFromMap(map[string]any, actions.AdvancedActionInterface) (actions.ActionInterface, error)
}

type serializer struct {
	iSerializer ISerializer
}

var (
	GobSerializer   = sGob{}
	JsonSerializer  = sJson{}
	YamlSerializer  = sYaml{}
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
		action = actions.NewClick(rawMap["button"].(string))
	case "move":
		action = actions.NewMove(rawMap["x"].(int), rawMap["y"].(int))
	case "key":
		action = actions.NewKey(rawMap["key"].(string), rawMap["state"].(string))
	case "imagesearch":
		targets := make([]string, 0)
		for _, t := range rawMap["targets"].([]any) {
			targets = append(targets, t.(string))
		}
		action = actions.NewImageSearch(rawMap["name"].(string), []actions.ActionInterface{}, targets, createSearchBox(rawMap["searcharea"].(map[string]any)))
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
	log.Printf("Unmarshalled action %s", action)
	return action, nil
}

func createSearchBox(rawMap map[string]any) data.SearchArea {
	return data.SearchArea{
		Name:    rawMap["name"].(string),
		LeftX:   rawMap["leftx"].(int),
		TopY:    rawMap["topy"].(int),
		RightX:  rawMap["rightx"].(int),
		BottomY: rawMap["bottomy"].(int),
	}
}
