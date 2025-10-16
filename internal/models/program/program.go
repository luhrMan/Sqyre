package program

import (
	"Squire/internal/config"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/items"
	"log"
)

var errStr = "unable to decode into struct, %v"

type Program struct {
	Name        string
	Items       *items.Items
	Coordinates map[string]*coordinates.Coordinates
}

func (p *Program) GetItems() *items.Items {
	return p.Items
}

func (p *Program) GetItemsMap() map[string]*items.Item {
	return p.Items.Items
}
func GetProgram(s string) *Program {
	var (
		keyStr = "programs" + "." + s + "."
		err    error
	)
	var p = new(Program)
	err = config.ViperConfig.UnmarshalKey(keyStr+"name", &p.Name)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = config.ViperConfig.UnmarshalKey(keyStr+"items", &p.Items)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = config.ViperConfig.UnmarshalKey(keyStr+"coordinates", &p.Coordinates)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	return p
}

func GetPrograms() map[string]*Program {
	var (
		ps = make(map[string]*Program)
		ss = config.ViperConfig.GetStringMap("programs")
	)
	for s := range ss {
		var (
			p = new(Program)
		)
		p = GetProgram(s)
		ps[s] = p
	}
	log.Println("programs loaded", ps)
	return ps
}

func NewProgram(name string) *Program {
	return &Program{
		Name:  name,
		Items: &items.Items{},
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
