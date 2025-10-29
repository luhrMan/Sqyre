package models

import (
	"Squire/internal/config"
	"fmt"
	"slices"
	"strings"
)

func (p *Program) GetItem(i string) (*Item, error) {
	if item, ok := p.Items[strings.ToLower(i)]; ok {
		return item, nil
	}
	return &Item{}, fmt.Errorf("item does not exist")
	// index, found := slices.BinarySearch(SortByName(allItemsMap), i)
	// if found {
	// 	return allItemsMap[AllItems()[index]], nil
	// }
}

func (p *Program) GetItemsWithAppendedProgramName() map[string]*Item {
	is := make(map[string]*Item)
	for s, i := range p.Items {
		s = p.Name + config.ProgramDelimiter + s
		is[s] = i
	}
	return is
}

func (p *Program) GetItemsAsStringSlice() []string {
	items := []string{}
	for _, i := range p.Items {
		items = append(items, strings.ToLower(i.Name))
	}
	return items
}

func (p *Program) SortItemsByName() []string {
	items := []string{}
	for _, i := range p.Items {
		items = append(items, strings.ToLower(i.Name))
	}
	if !slices.IsSorted(items) {
		slices.Sort(items)
	}
	return items
}

func (p *Program) GetItemsMap() map[string]*Item {
	return p.Items
}

func (p *Program) AddItem(i *Item) {
	p.Items[strings.ToLower(i.Name)] = i
}

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
