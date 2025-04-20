package items

import (
	"errors"
)

type Item struct {
	Name     string `json:"name"`
	GridSize [2]int `json:"gridSize"`
	StackMax int    `json:"stackMax"`
	Merchant string `json:"merchant"`
}

type ItemsMap struct {
	Map map[string][]Item
}

func (is *ItemsMap) GetItemsMapAsStringsMap() map[string][]string {
	itemsStringMap := make(map[string][]string)
	for str, items := range is.Map {
		names := make([]string, len(items))
		for i, item := range items {
			names[i] = item.Name
		}
		itemsStringMap[str] = names
	}
	return itemsStringMap
}

// func (is *ItemsMap) GetItemsMapCategory(category string) *[]string {
// 	im := is.Map
// 	keys := make([]string, 0, len(im[category]))
// 	for _, k := range im[category] {
// 		keys = append(keys, k.Name)
// 	}
// 	return &keys
// }

func (is *ItemsMap) GetItem(key string) (*Item, error) {
	for _, items := range is.Map {
		for _, item := range items {
			if item.Name == key {
				return &item, nil
			}
		}
	}
	return nil, errors.New("could not find item")
}
