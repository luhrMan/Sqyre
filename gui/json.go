package gui

import (
	"Dark-And-Darker/structs"
	"encoding/json"
	"fmt"
	"log"
	"os"
)

func (m *macroTree) saveTreeToJsonFile(filename string) error {
	// Marshal the action to JSON
	jsonData, err := json.MarshalIndent(m.root, "", "\t")
	if err != nil {
		return fmt.Errorf("error marshalling tree: %v", err)
	}
	filepath := "./saved-macros/" + filename + ".json"
	// Write the JSON data to the file
	err = os.WriteFile(filepath, jsonData, 0644)
	if err != nil {
		return fmt.Errorf("error writing to file: %v", err)
	}
	return nil
}

func (m *macroTree) loadTreeFromJsonFile(filename string) error {
	log.Printf("loadTreeFromJsonFile: attempting to read file %v", filename)
	jsonData, err := os.ReadFile("./saved-macros/" + filename)
	if err != nil {
		return fmt.Errorf("error reading file: %v", err)
	}
	var result structs.ActionInterface
	//err = json.Unmarshal(jsonData, root)
	m.root.SubActions = []structs.ActionInterface{}
	result, err = UnmarshalJSON(jsonData)
	if s, ok := result.(*structs.LoopAction); ok { // fill root / tree
		for _, sa := range s.SubActions {
			m.root.AddSubAction(sa)
		}
	}
	if err != nil {
		return fmt.Errorf("error unmarshalling tree: %v", err)
	}
	macro.tree.Refresh()
	return err
}

// func encodeToGobFile(data *structs.LoopAction, filename string) {
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

// func decodeFromGobFile(filename string) structs.LoopAction {

// 	// Open file for reading
// 	file, err := os.Open(filename)
// 	if err != nil {
// 		fmt.Println("Error opening file:", err)
// 		// return structs.LoopAction{}
// 	}
// 	defer file.Close()

// 	// Create a new decoder and decode the data
// 	var data structs.LoopAction
// 	decoder := gob.NewDecoder(file)

// 	if err := decoder.Decode(&data); err != nil {
// 		fmt.Println("Error decoding data:", err)
// 		// return structs.LoopAction{}
// 	}

// 	return data
// }

// // **** NEW ATTEMPT
func UnmarshalJSON(data []byte) (structs.ActionInterface, error) {
	var rawMap map[string]interface{}
	err := json.Unmarshal(data, &rawMap)
	if err != nil {
		return nil, err
	}

	return createActionFromMap(rawMap, nil)
}

func createActionFromMap(rawMap map[string]interface{}, parent structs.AdvancedActionInterface) (structs.ActionInterface, error) {
	log.Println(rawMap)
	var action structs.ActionInterface
	// if parent != nil {

	// } else {
	// 	rawMap["name"] != nil:
	// 	parent.SetName()
	// }
	switch {
	case rawMap["loopcount"] != nil:
		action = structs.NewLoopAction(int(rawMap["loopcount"].(float64)), "", []structs.ActionInterface{})
		// if baseActionMap, ok := rawMap["baseaction"].(map[string]interface{}); ok {
		// 	uid := baseActionMap["uid"].(string)
		// 	action.UpdateBaseAction(uid, parent)
		// }
	case rawMap["waittime"] != nil:
		action = structs.NewWaitAction(int(rawMap["waittime"].(float64)))
	case rawMap["button"] != nil:
		action = structs.NewClickAction(rawMap["button"].(string))
	case rawMap["X"] != nil && rawMap["Y"] != nil:
		action = structs.NewMoveAction(int(rawMap["X"].(float64)), int(rawMap["Y"].(float64)))
	case rawMap["key"] != nil:
		action = structs.NewKeyAction(rawMap["key"].(string), rawMap["state"].(string))
	case rawMap["imagetargets"] != nil:
		targets := make([]string, 0)
		for _, t := range rawMap["imagetargets"].([]interface{}) {
			targets = append(targets, t.(string))
		}
		action = structs.NewImageSearchAction("", []structs.ActionInterface{}, targets, createSearchBox(rawMap["searchbox"].(map[string]interface{})))
	case rawMap["texttarget"] != nil:
		action = &structs.OcrAction{
			AdvancedAction: structs.AdvancedAction{
				BaseAction: structs.NewBaseAction(),
			},
			Target:    rawMap["texttarget"].(string),
			SearchBox: createSearchBox(rawMap["searchbox"].(map[string]interface{})),
		}
	default:
		//action = &structs.AdvancedAction{}
	}
	// if advActMap, ok := rawMap["advancedaction"].(map[string]interface{}); ok {
	// 	if baseActionMap, ok := advActMap["baseaction"].(map[string]interface{}); ok {
	// 		uid := baseActionMap["uid"].(string)
	// 		action.UpdateBaseAction(uid, parent)
	// 	}
	// }
	// Set BaseAction
	if baseActionMap, ok := rawMap["baseaction"].(map[string]interface{}); ok {
		uid := baseActionMap["uid"].(string)
		action.UpdateBaseAction(uid, parent)
	}
	if uid, ok := rawMap["uid"].(string); ok {
		action.SetUID(uid)
	}
	// Handle AdvancedAction fields
	if advAction, ok := action.(structs.AdvancedActionInterface); ok {
		log.Println("Advanced Action unmarshal")
		if name, ok := rawMap["name"].(string); ok {
			advAction.SetName(name)
		}

		if subActionsRaw, ok := rawMap["subactions"].([]interface{}); ok {
			log.Println("SubActions unmarshal")
			// var subActionList []structs.ActionInterface
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
