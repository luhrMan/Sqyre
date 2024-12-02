package structs

import (
	"embed"
	"encoding/json"
	"errors"
	"log"
	"os"
)

type Item struct {
	Name     string `json:"name"`
	GridSize [2]int `json:"gridSize"`
	StackMax int    `json:"stackMax"`
	Merchant string `json:"merchant"`
}

type Items struct {
	Map map[string][]Item
}

//go:embed items.json
var items embed.FS

func (is *Items) CreateItemMaps() {
	log.Println("GetItemsMap()")
	is.Map = make(map[string][]Item)

	log.Println("Initializing Items Map")

	file, err := os.Open("./json-data/items.json")
	if err != nil {
		log.Println("Error opening file:", err)
		panic(err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	if err := decoder.Decode(&is.Map); err != nil {
		log.Println("Error decoding JSON:", err)
		panic(err)
	}
}

func (is *Items) GetItemsMapAsStringsMap() map[string][]string {
	itemsStringMap := make(map[string][]string)
	for str, items := range is.Map {
		names := make([]string, len(items))
		for i, item := range items {
			names[i] = item.Name
		}
		//		log.Printf("strMap add names: %v", names)
		itemsStringMap[str] = names
	}
	return itemsStringMap
}

func (is *Items) GetItemsMapCategory(category string) *[]string {
	im := is.Map
	keys := make([]string, 0, len(im[category]))
	for _, k := range im[category] {
		keys = append(keys, k.Name)
	}
	return &keys
}

func (is *Items) GetItem(key string) (*Item, error) {
	for _, items := range is.Map {
		for _, item := range items {
			if item.Name == key {
				return &item, nil
			}
		}
	}
	return nil, errors.New("could not find item")
}
