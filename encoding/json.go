package encoding

import (
	"Squire/internal/config"
	"encoding/json"
	"fmt"
	"log"
	"os"
)

type sJson struct {
	serializer
}

func (s *sJson) Encode(filename string, d any) error {
	filename += config.JSON
	if filename == "" {
		return fmt.Errorf("cannot save empty filename")
	}
	jsonData, err := json.MarshalIndent(d, "", "\t")
	if err != nil {
		return fmt.Errorf("error marshalling tree: %v", err)
	}
	err = os.WriteFile(filename, jsonData, 0644)
	if err != nil {
		return fmt.Errorf("error writing to file: %v", err)
	}
	return nil
}

func (s *sJson) Decode(filename string) (any, error) {
	filename += config.JSON
	log.Printf("Json Decoding: attempting to read file %v", filename)
	jsonData, err := os.ReadFile(filename)
	if err != nil {
		return nil, fmt.Errorf("error reading file: %v", err)
	}
	log.Println("File read: ", filename)

	var rawMap map[string]any
	err = json.Unmarshal(jsonData, &rawMap)
	if err != nil {
		return nil, fmt.Errorf("error unmarhsalling json file: %v", err)
	}

	return rawMap, err

}

///////////////////////////////////////////////////////////////////
// var path = "./internal/saved-macros/"

// func (m *macro) saveTreeToJsonFile(filename string) error {
// 	if filename == "" {
// 		return fmt.Errorf("cannot save empty filename")
// 	}
// 	jsonData, err := json.MarshalIndent(m.root, "", "\t")
// 	if err != nil {
// 		return fmt.Errorf("error marshalling tree: %v", err)
// 	}
// 	filepath := path + filename + ".json"
// 	err = os.WriteFile(filepath, jsonData, 0644)
// 	if err != nil {
// 		return fmt.Errorf("error writing to file: %v", err)
// 	}
// 	return nil
// }

// func (m *macro) loadTreeFromJsonFile(filename string) error {
// 	log.Printf("loadTreeFromJsonFile: attempting to read file %v", filename)
// 	jsonData, err := os.ReadFile(path + filename)
// 	if err != nil {
// 		return fmt.Errorf("error reading file: %v", err)
// 	}
// 	var result actions.ActionInterface
// 	//err = json.Unmarshal(jsonData, root)
// 	m.root.SubActions = []actions.ActionInterface{}
// 	result, err = UnmarshalJSON(jsonData)
// 	if s, ok := result.(*actions.Loop); ok { // fill root / tree
// 		for _, sa := range s.SubActions {
// 			m.root.AddSubAction(sa)
// 		}
// 	}
// 	if err != nil {
// 		return fmt.Errorf("error unmarshalling tree: %v", err)
// 	}
// 	m.tree.Refresh()
// 	return err
// }

// func UnmarshalJSON(config []byte) (actions.ActionInterface, error) {
// 	var rawMap map[string]interface{}
// 	err := json.Unmarshal(config, &rawMap)
// 	if err != nil {
// 		return nil, err
// 	}

// 	return createActionFromMap(rawMap, nil)
// }
