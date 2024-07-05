package structs

import (
	"encoding/json"
	"log"
	"os"
	"sync"
)

type SearchBoxes struct {
	Boxes []SearchBox `json:"searchBoxes"`
}

type SearchBox struct {
	Name       string `json:"name"`
	SearchArea struct {
		LeftX   int `json:"x1"`
		TopY    int `json:"y1"`
		RightX  int `json:"x2"`
		BottomY int `json:"y2"`
	} `json:"searchArea"`
}

type Spots struct {
	Spots []Spot `json:"spots"`
}

type Spot struct {
	Name        string `json:"name"`
	Coordinates struct {
		X int `json:"x"`
		Y int `json:"y"`
	} `json:"coordinates"`
}

var (
	sbMap       *map[string]SearchBox
	sbOnce      sync.Once
	spotMap     *map[string]Spot
	spotMapOnce sync.Once
)

//var searchBoxes *SearchBoxes
//ar searchBoxMap = make(map[string]SearchBox)
//var spots *Spots
//var spotMap = make(map[string]Spot)

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
		file, err := os.Open("./json-data/searchBoxes.json")
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

		log.Println("Search Coordinates:")
		for _, sbArr := range tempArrMap {
			for _, sb := range sbArr {
				log.Printf("Area: %s, X1: %d Y1: %d X2: %d Y2: %d\n", sb.Name, sb.SearchArea.LeftX, sb.SearchArea.TopY, sb.SearchArea.RightX, sb.SearchArea.BottomY)
				tempMap[sb.Name] = sb
			}
		}
		sbMap = &tempMap
	})
	return sbMap
}

func GetSpot(key string) *Spot {
	m := *GetSpotMap()
	s := m[key]
	return &s
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
		file, err := os.Open("./json-data/spots.json")
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

		// for _, spot := range *spotMap {
		// 	spotMap[spot.Name] = spot
		// }

		// Print out the decoded data
		log.Println("Search Coordinates:")
		for _, sArr := range tempArrMap {
			for _, s := range sArr {
				log.Printf("Spot: %s, X: %d Y: %d\n", s.Name, s.Coordinates.X, s.Coordinates.Y)
				tempMap[s.Name] = s
			}
		}
		spotMap = &tempMap
	})
	return spotMap
}
