package main

import (
	"Squire/internal/actions"
	"Squire/internal/structs"
	"encoding/json"
	"fmt"
	"log"
	"os"
)

var path = "./internal/saved-macros/"

func (m *macro) saveTreeToJsonFile(filename string) error {
	if filename == "" {
		return fmt.Errorf("cannot save empty filename")
	}
	jsonData, err := json.MarshalIndent(m.root, "", "\t")
	if err != nil {
		return fmt.Errorf("error marshalling tree: %v", err)
	}
	filepath := path + filename + ".json"
	err = os.WriteFile(filepath, jsonData, 0644)
	if err != nil {
		return fmt.Errorf("error writing to file: %v", err)
	}
	return nil
}

func (m *macro) loadTreeFromJsonFile(filename string) error {
	log.Printf("loadTreeFromJsonFile: attempting to read file %v", filename)
	jsonData, err := os.ReadFile(path + filename)
	if err != nil {
		return fmt.Errorf("error reading file: %v", err)
	}
	var result actions.ActionInterface
	//err = json.Unmarshal(jsonData, root)
	m.root.SubActions = []actions.ActionInterface{}
	result, err = UnmarshalJSON(jsonData)
	if s, ok := result.(*actions.Loop); ok { // fill root / tree
		for _, sa := range s.SubActions {
			m.root.AddSubAction(sa)
		}
	}
	if err != nil {
		return fmt.Errorf("error unmarshalling tree: %v", err)
	}
	m.tree.Refresh()
	return err
}

// func encodeToGobFile(data *actions.LoopAction, filename string) {
// 	file, err := os.Create(filename)
// 	if err != nil {
// 		fmt.Println("Error creating file:", err)
// 		return
// 	}
// 	defer file.Close()

// 	// Create a new encoder and encode the data
// 	encoder := gob.NewEncoder(file)
// 	if err := encoder.Encode(data); err != nil {
// 		fmt.Println("Error encoding data:", err)
// 		return
// 	}

// 	fmt.Println("Data encoded and saved to", filename)
// }

// func decodeFromGobFile(filename string) actions.LoopAction {

// 	// Open file for reading
// 	file, err := os.Open(filename)
// 	if err != nil {
// 		fmt.Println("Error opening file:", err)
// 		// return actions.LoopAction{}
// 	}
// 	defer file.Close()

// 	// Create a new decoder and decode the data
// 	var data actions.LoopAction
// 	decoder := gob.NewDecoder(file)

// 	if err := decoder.Decode(&data); err != nil {
// 		fmt.Println("Error decoding data:", err)
// 		// return actions.LoopAction{}
// 	}

// 	return data
// }

// // **** NEW ATTEMPT
func UnmarshalJSON(data []byte) (actions.ActionInterface, error) {
	var rawMap map[string]interface{}
	err := json.Unmarshal(data, &rawMap)
	if err != nil {
		return nil, err
	}

	return createActionFromMap(rawMap, nil)
}

func createActionFromMap(rawMap map[string]interface{}, parent actions.AdvancedActionInterface) (actions.ActionInterface, error) {
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
		for _, t := range rawMap["imagetargets"].([]interface{}) {
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
	if baseActionMap, ok := rawMap["baseaction"].(map[string]interface{}); ok {
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
				subAction, err := createActionFromMap(subActionRaw.(map[string]interface{}), advAction)
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

func createSearchBox(rawMap map[string]interface{}) structs.SearchBox {
	return structs.SearchBox{
		Name:    rawMap["name"].(string),
		LeftX:   int(rawMap["x1"].(float64)),
		TopY:    int(rawMap["y1"].(float64)),
		RightX:  int(rawMap["x2"].(float64)),
		BottomY: int(rawMap["y2"].(float64)),
	}
}
