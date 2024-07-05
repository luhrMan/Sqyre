package structs

import (
	"encoding/json"
	"log"
	"os"
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

var searchBoxes *SearchBoxes
var searchBoxMap = make(map[string]SearchBox)
var spots *Spots
var spotMap = make(map[string]Spot)

func GetSearchBox(key string) SearchBox {
	return searchBoxMap[key]
}

func GetSearchBoxMapKeys(m map[string]SearchBox) *[]string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return &keys
}

func GetSearchBoxMap() *map[string]SearchBox {
	return &searchBoxMap
}

func SearchBoxMapInit() *map[string]SearchBox {
	file, err := os.Open("./json-data/searchBoxes.json")
	if err != nil {
		log.Println("Error opening file:", err)
		panic(err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	if err := decoder.Decode(&searchBoxes); err != nil {
		log.Println("Error decoding JSON:", err)
		panic(err)
	}

	for _, box := range searchBoxes.Boxes {
		searchBoxMap[box.Name] = box
	}
	log.Println("Search Coordinates:")
	for _, s := range searchBoxMap {
		log.Printf("Area: %s, X1: %d Y1: %d X2: %d Y2: %d\n", s.Name, s.SearchArea.LeftX, s.SearchArea.TopY, s.SearchArea.RightX, s.SearchArea.BottomY)
	}
	return &searchBoxMap
}

func GetSpot(key string) Spot {
	return spotMap[key]
}

func GetSpotMap() *map[string]Spot {
	return &spotMap
}

func GetSpotMapKeys(m map[string]Spot) *[]string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return &keys
}

func SpotMapInit() *map[string]Spot {
	file, err := os.Open("./json-data/spots.json")
	if err != nil {
		log.Println("Error opening file:", err)
		panic(err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	if err := decoder.Decode(&spots); err != nil {
		log.Println("Error decoding JSON:", err)
		panic(err)
	}

	for _, spot := range spots.Spots {
		spotMap[spot.Name] = spot
	}

	// Print out the decoded data
	log.Println("Search Coordinates:")
	for _, s := range spotMap {
		log.Printf("Spot: %s, X: %d Y: %d\n", s.Name, s.Coordinates.X, s.Coordinates.Y)
	}
	return &spotMap
}
