package structs

import (
	"encoding/json"
	"log"
	"os"
	"sync"
)

type SearchBox struct {
	Name    string `json:"name"`
	LeftX   int    `json:"x1"`
	TopY    int    `json:"y1"`
	RightX  int    `json:"x2"`
	BottomY int    `json:"y2"`
}

type Spot struct {
	Name string `json:"name"`
	X    int    `json:"x"`
	Y    int    `json:"y"`
}

var (
	path        = "./internal/resources/json/"
	sbMap       *map[string]SearchBox
	sbOnce      sync.Once
	spotMap     *map[string]Spot
	spotMapOnce sync.Once
)

func GetSearchBox(key string) *SearchBox {
	m := *GetSearchBoxMap()
	sb := m[key]
	return &sb
}

func GetSearchBoxMapKeys(m map[string]SearchBox) *[]string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return &keys
}

func GetSearchBoxMap() *map[string]SearchBox {
	sbOnce.Do(func() {
		log.Println("Initializing Searchbox Map")
		tempArrMap := make(map[string][]SearchBox)
		tempMap := make(map[string]SearchBox)
		file, err := os.Open(path + "searchBoxes.json")
		if err != nil {
			log.Println("Error opening file:", err)
			panic(err)
		}
		defer file.Close()

		decoder := json.NewDecoder(file)
		if err := decoder.Decode(&tempArrMap); err != nil {
			log.Println("Error decoding JSON:", err)
			panic(err)
		}

		//		log.Println("Search Coordinates:")
		for _, sbArr := range tempArrMap {
			for _, sb := range sbArr {
				//				log.Printf("Area: %s, X1: %d Y1: %d X2: %d Y2: %d\n", sb.Name, sb.LeftX, sb.TopY, sb.RightX, sb.BottomY)
				tempMap[sb.Name] = sb
			}
		}
		sbMap = &tempMap
	})
	return sbMap
}

func GetSpot(key string) *Spot {
	m := *GetSpotMap()
	if s, ok := m[key]; ok {
		//		s := m[key]
		return &s
	}
	return nil
}

func GetSpotMapKeys(m map[string]Spot) *[]string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return &keys
}

func GetSpotMap() *map[string]Spot {
	spotMapOnce.Do(func() {
		tempArrMap := make(map[string][]Spot)
		tempMap := make(map[string]Spot)

		file, err := os.Open(path + "spots.json")
		if err != nil {
			log.Println("Error opening file:", err)
			panic(err)
		}
		defer file.Close()

		decoder := json.NewDecoder(file)
		if err := decoder.Decode(&tempArrMap); err != nil {
			log.Println("Error decoding JSON:", err)
			panic(err)
		}

		for _, sArr := range tempArrMap {
			for _, s := range sArr {
				//				log.Printf("Spot: %s, X: %d Y: %d\n", s.Name, s.X, s.Y)
				tempMap[s.Name] = s
			}
		}
		spotMap = &tempMap
	})
	return spotMap
}

func GetSpotJsonMap() map[string][]Spot {
	spotJsonMap := make(map[string][]Spot)

	file, err := os.Open(path + "spots.json")
	if err != nil {
		log.Println("Error opening file:", err)
		panic(err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	if err := decoder.Decode(&spotJsonMap); err != nil {
		log.Println("Error decoding JSON:", err)
		panic(err)
	}
	return spotJsonMap
}

func GetSpotMapAsStringsMap() *map[string][]string {
	spotStringsMap := make(map[string][]string)
	jsonMap := GetSpotJsonMap()
	for str, items := range jsonMap {
		names := make([]string, len(items))
		for i, item := range items {
			names[i] = item.Name
		}
		//		log.Printf("strMap add names: %v", names)
		spotStringsMap[str] = names
		spotStringsMap[""] = append(spotStringsMap[""], str)
	}
	return &spotStringsMap
}
