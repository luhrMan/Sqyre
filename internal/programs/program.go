package model

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"log"
)

var errStr = "unable to decode into struct, %v"

type Program struct {
	Name        string
	Items       map[string]items.Item
	Coordinates map[string]*coordinates.Coordinates
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

	// macros := config.ViperConfig.GetStringSlice(keyStr + "macros")
	// for i := range macros {
	// 	p.Macros = append(p.Macros, macro.NewMacro("New Macro "+strconv.Itoa(i), 30, []string{}))
	// 	err := p.GetMacroAtIndex(i).UnmarshalMacro(i)
	// 	if err != nil {
	// 		log.Println(err)
	// 	}
	// }
	// log.Println("Program data loaded from config.yaml:", p.Name)
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
	// log.Println("dark and darker shit", *ps[config.DarkAndDarker])
	return ps
}

func NewProgram(name string) *Program {
	return &Program{
		Name: name,
		// Macros: []*macro.Macro{},
		Items: make(map[string]items.Item),
		// Coordinates: map[string]*coordinates.Coordinates{
		// 	strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
		// 		Points:      make(map[string]coordinates.Point),
		// 		SearchAreas: make(map[string]coordinates.SearchArea),
		// 	},
		// },
	}
}

// func (p *Program) GetMacroAtIndex(i int) *macro.Macro { return p.Macros[i] }

// func (p *Program) GetItem(i string) items.Item {
// 	if item, ok := p.Items[i]; ok {
// 		return item
// 	}
// 	return items.Item{}
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
