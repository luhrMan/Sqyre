package structs

import (
	"encoding/json"
	"errors"
	"log"
	"os"
)

type ItemsByCategory struct {
	Categories map[string][]Item `json:"categories"`
}
type Item struct {
	Name     string `json:"name"`
	GridSize [2]int `json:"gridSize"`
	StackMax int    `json:"stackMax"`
	Merchant string `json:"merchant"`
}

func ItemsByCategoryMap() *ItemsByCategory {
	// Open the JSON file
	file, err := os.Open("./json-data/items.json")
	if err != nil {
		log.Println("Error opening file:", err)
		panic(err)
	}
	defer file.Close()

	// Decode JSON from the file into a struct
	// Create a JSON decoder and decode the file contents
	itemsByCategory := ItemsByCategory{}
	decoder := json.NewDecoder(file)
	if err := decoder.Decode(&itemsByCategory); err != nil {
		log.Println("Error decoding JSON:", err)
		panic(err)
	}

	// Print out the decoded data
	//	log.Println("Items:")
	//	for category, items := range itemsByCategory.Categories {
	//		log.Printf("- Category: %s\n", category)
	//		for _, item := range items {
	//			log.Printf("  - Name: %s, GridSize: %v, StackMax: %d, Merchant: %s\n", item.Name, item.GridSize, item.StackMax, item.Merchant)
	//		}
	//	}
	return &itemsByCategory
}

func GetItem(key string) (*Item, error) {
	itemsByCategory := *ItemsByCategoryMap()
	for _, items := range itemsByCategory.Categories {
		for _, item := range items {
			if item.Name == key {
				return &item, nil
			}
		}
	}
	return nil, errors.New("could not find item")
}
