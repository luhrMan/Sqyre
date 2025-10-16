package items

import (
	"fmt"
	"slices"
	"sort"
	"strings"
)

// var (
// 	allItemsSlice            []string
// 	allItemsSortedByName     []string
// 	allItemsSortedByCategory []string
// 	allItemsMap              = make(map[string]Item)
// )

type Items struct {
	Items map[string]*Item
}
type Item struct {
	Name     string `json:"name"`
	GridSize [2]int `json:"gridSize"`
	Category string `json:"category"`
	StackMax int    `json:"stackMax"`
	Merchant string `json:"merchant"`
}

func (is *Items) GetItem(i string) (*Item, error) {
	if item, ok := is.Items[strings.ToLower(i)]; ok {
		return item, nil
	}
	return &Item{}, fmt.Errorf("item does not exist")
	// index, found := slices.BinarySearch(SortByName(allItemsMap), i)
	// if found {
	// 	return allItemsMap[AllItems()[index]], nil
	// }
}

// func ParseItemsFromJson(path string) []Item {
// 	im := []Item{}
// 	content, err := os.ReadFile(path)
// 	if err != nil {
// 		log.Println("Error when opening file: ", err)
// 		return nil
// 	}
// 	err = json.Unmarshal(content, &im)
// 	if err != nil {
// 		log.Printf("Error unmarshaling JSON: %v\n", err)
// 		return nil
// 	}
// 	log.Println(im)
// 	return im
// }

func (is *Items) SortByCategory() []string {
	categories := make([]string, 0, len(is.Items))
	items := []string{}
	for _, i := range is.Items {
		if !slices.Contains(categories, i.Category) {
			categories = append(categories, i.Category)
		}
	}
	sort.Strings(categories)
	for _, c := range categories {
		for _, i := range is.SortByName() {
			if is.Items[strings.ToLower(i)].Category == c {
				items = append(items, is.Items[strings.ToLower(i)].Name)
			}
		}
	}
	return items
}

func (is *Items) SortByName() []string {
	items := []string{}
	for _, i := range is.Items {
		items = append(items, i.Name)
	}
	if !slices.IsSorted(items) {
		slices.Sort(items)
	}
	return items
}

// func SetAllItems(is []string) {
// 	allItemsSlice = is
// }

// func AllItems(sortedby string) []string {
// 	switch sortedby {
// 	case "none":
// 		return allItemsSlice
// 	case "category":
// 		return allItemsSortedByCategory
// 	case "name":
// 		return allItemsSortedByName
// 	default:
// 		return allItemsSlice
// 	}
// }

// func SetItemsMap(ism map[string]Item) {
// 	allItemsMap = ism
// 	allItemsSortedByName = SortByName(ism)
// 	allItemsSortedByCategory = SortByCategory(ism)
// 	allItemsSlice = allItemsSortedByName
// }

// func ItemsMap() map[string]Item {
// 	return allItemsMap
// }

// func (is *ItemsMap) GetItemsMapAsStringsMap() map[string][]string {
// 	itemsStringMap := make(map[string][]string)
// 	for str, items := range is.Map {
// 		names := make([]string, len(items))
// 		for i, item := range items {
// 			names[i] = item.Name
// 		}
// 		itemsStringMap[str] = names
// 	}
// 	return itemsStringMap
// }

// func (is *ItemsMap) GetItemsMapCategory(category string) *[]string {
// 	im := is.Map
// 	keys := make([]string, 0, len(im[category]))
// 	for _, k := range im[category] {
// 		keys = append(keys, k.Name)
// 	}
// 	return &keys
// }

// func (is *ItemsMap) GetItem(key string) (*Item, error) {
// 	for _, items := range is.Map {
// 		for _, item := range items {
// 			if item.Name == key {
// 				return &item, nil
// 			}
// 		}
// 	}
// 	return nil, errors.New("could not find item")
// }
