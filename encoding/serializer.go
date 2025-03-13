package encoding

import (
	"Squire/internal/actions"
	"Squire/internal/structs"
	"log"
)

type ISerializer interface {
	Encode(any, string) error
	Decode(string) (map[string]any, error)
	CreateActionFromMap(map[string]any, actions.AdvancedActionInterface) (actions.ActionInterface, error)
}

type serializer struct {
	iSerializer ISerializer
}

var (
	GobSerializer  = sGob{}
	JsonSerializer = sJson{}
	Serializer     = serializer{}.iSerializer
)

func (s *serializer) CreateActionFromMap(rawMap map[string]any, parent actions.AdvancedActionInterface) (actions.ActionInterface, error) {
	log.Println(rawMap)
	var action actions.ActionInterface
	// if parent != nil {

	// } else {
	// 	rawMap["name"] != nil:
	// 	parent.SetName()
	// }
	switch {
	case rawMap["loopcount"] != nil:
		action = actions.NewLoop(int(rawMap["loopcount"].(float64)), "", []actions.ActionInterface{})
	case rawMap["waittime"] != nil:
		action = actions.NewWait(int(rawMap["waittime"].(float64)))
	case rawMap["button"] != nil:
		action = actions.NewClick(rawMap["button"].(string))
	case rawMap["X"] != nil && rawMap["Y"] != nil:
		action = actions.NewMove(int(rawMap["X"].(float64)), int(rawMap["Y"].(float64)))
	case rawMap["key"] != nil:
		action = actions.NewKey(rawMap["key"].(string), rawMap["state"].(string))
	case rawMap["imagetargets"] != nil:
		targets := make([]string, 0)
		for _, t := range rawMap["imagetargets"].([]any) {
			targets = append(targets, t.(string))
		}
		action = actions.NewImageSearch("", []actions.ActionInterface{}, targets, createSearchBox(rawMap["searchbox"].(map[string]interface{})))
	case rawMap["texttarget"] != nil:
		action = actions.NewOcr("", []actions.ActionInterface{}, rawMap["texttarget"].(string), createSearchBox(rawMap["searchbox"].(map[string]interface{})))
	default:
		//action = &actions.advancedAction{}
	}
	// if advActMap, ok := rawMap["advancedaction"].(map[string]interface{}); ok {
	// 	if baseActionMap, ok := advActMap["baseaction"].(map[string]interface{}); ok {
	// 		uid := baseActionMap["uid"].(string)
	// 		action.UpdateBaseAction(uid, parent)
	// 	}
	// }
	// Set baseAction
	if baseActionMap, ok := rawMap["baseaction"].(map[string]any); ok {
		uid := baseActionMap["uid"].(string)
		action.UpdateBaseAction(uid, parent)
	}
	if uid, ok := rawMap["uid"].(string); ok {
		action.SetUID(uid)
	}
	// Handle advancedAction fields
	if advAction, ok := action.(actions.AdvancedActionInterface); ok {
		log.Println("Advanced Action unmarshal")
		if name, ok := rawMap["name"].(string); ok {
			advAction.SetName(name)
		}

		if subActionsRaw, ok := rawMap["subactions"].([]interface{}); ok {
			log.Println("SubActions unmarshal")
			// var subActionList []actions.ActionInterface
			for _, subActionRaw := range subActionsRaw {
				subAction, err := s.CreateActionFromMap(subActionRaw.(map[string]interface{}), advAction)
				if err != nil {
					return nil, err
				}
				advAction.AddSubAction(subAction)
				//				log.Println(subAction.GetParent())

				// subActionList = append(subActionList, subAction)
			}
			//updateTree(&tree, root)
			// for _, sa := range subActionList {
			// 	advAction.AddSubAction(sa)
			// }
		}
	}
	log.Printf("Unmarshalled action %s", action)
	return action, nil
}

func createSearchBox(rawMap map[string]interface{}) structs.SearchArea {
	return structs.SearchArea{
		Name:    rawMap["name"].(string),
		LeftX:   int(rawMap["x1"].(float64)),
		TopY:    int(rawMap["y1"].(float64)),
		RightX:  int(rawMap["x2"].(float64)),
		BottomY: int(rawMap["y2"].(float64)),
	}
}
