package program

import (
	"Squire/internal/config"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/items"
	"Squire/internal/models/serialize"
	"fmt"
	"log"
	"slices"
	"strings"
)

type Program struct {
	Name        string
	Items       map[string]*items.Item
	Coordinates map[string]*coordinates.Coordinates
}

func (p *Program) GetItemsWithAppendedProgramName() map[string]*items.Item {
	is := make(map[string]*items.Item)
	for s, i := range p.Items {
		s = p.Name + config.ProgramDelimiter + s
		is[s] = i
	}
	return is
}

func (p *Program) GetItem(i string) (*items.Item, error) {
	if item, ok := p.Items[strings.ToLower(i)]; ok {
		return item, nil
	}
	return &items.Item{}, fmt.Errorf("item does not exist")
	// index, found := slices.BinarySearch(SortByName(allItemsMap), i)
	// if found {
	// 	return allItemsMap[AllItems()[index]], nil
	// }
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

func (p *Program) GetItemsMap() map[string]*items.Item {
	return p.Items
}

func NewProgram(name string) *Program {
	return &Program{
		Name:  name,
		Items: make(map[string]*items.Item),
		// Coordinates: map[string]*coordinates.Coordinates{
		// 	strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
		// 		Points:      make(map[string]coordinates.Point),
		// 		SearchAreas: make(map[string]coordinates.SearchArea),
		// 	},
		// },
	}
}

// func (p *Program) GetItem(i string) items.Item {
// 	if item, ok := p.Items[i]; ok {
// 		return item
// 	}
// 	return items.Item{}
// }

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

// func (p *Program) AddProgramPoint(ss string, point config.Point) {
// 	c := p.Coordinates
// 	points := c[ss].Points
// 	points[point.Name] = point
// }
// func (p *Program) SerializeJsonPointsToProgram(ss string) {
// 	jpm := config.JsonPointMap()
// 	for _, point := range jpm {
// 		p.AddProgramPoint(ss, point)
// 	}
// }

// func (p *Program) AddProgramSearchArea(ss string, sa config.SearchArea) {
// 	c := p.Coordinates
// 	sas := c[ss].SearchAreas
// 	sas[sa.Name] = sa
// }

func Decode(s string) *Program {
	var (
		keyStr = "programs" + "." + s + "."
		err    error
		errStr = "problem here lol"
	)

	var p = &Program{
		Items:       map[string]*items.Item{},
		Coordinates: map[string]*coordinates.Coordinates{},
	}
	err = serialize.GetViper().UnmarshalKey(keyStr+"name", &p.Name)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = serialize.GetViper().UnmarshalKey(keyStr+"items", &p.Items)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = serialize.GetViper().UnmarshalKey(keyStr+"coordinates", &p.Coordinates)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	return p
}

func DecodeAll() map[string]*Program {
	var (
		ps = make(map[string]*Program)
		ss = serialize.GetViper().GetStringMap("programs")
	)
	for s := range ss {
		p := Decode(s)
		ps[s] = p
	}
	log.Println("programs loaded", ps)
	return ps
}

func EncodeAll(d map[string]*Program) error {
	serialize.GetViper().Set("programs", d)
	err := serialize.GetViper().WriteConfig()
	if err != nil {
		return fmt.Errorf("error marshalling programs: %v", err)
	}
	log.Println("Successfully encoded programs")
	return nil
}
