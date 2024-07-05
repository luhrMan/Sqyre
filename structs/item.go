package structs

import (
	"encoding/json"
	"errors"
	"log"
	"os"
	"sync"
)

//	type ItemsByCategory struct {
//		Categories map[string][]Item `json:"categories"`
//	}
type Item struct {
	Name     string `json:"name"`
	GridSize [2]int `json:"gridSize"`
	StackMax int    `json:"stackMax"`
	Merchant string `json:"merchant"`
}

var (
	itemsMap     *map[string][]Item
	itemsMapOnce sync.Once
)

func GetItemsMap() *map[string][]Item {
	log.Println("ItemsMap Get")
	itemsMapOnce.Do(func() {
		tempMap := make(map[string][]Item)
		itemsMap = &tempMap
		log.Println("Initializing Items Map")

		file, err := os.Open("./json-data/items.json")
		if err != nil {
			log.Println("Error opening file:", err)
			panic(err)
		}
		defer file.Close()

		decoder := json.NewDecoder(file)
		if err := decoder.Decode(itemsMap); err != nil {
			log.Println("Error decoding JSON:", err)
			panic(err)
		}
	})
	return itemsMap
}

func GetItem(key string) (*Item, error) {
	for _, items := range *GetItemsMap() {
		for _, item := range items {
			if item.Name == key {
				return &item, nil
			}
		}
	}
	return nil, errors.New("could not find item")
}
