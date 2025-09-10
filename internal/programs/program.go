package programs

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"Squire/internal/programs/macro"
	"strconv"
)

type Programs map[string]*Program

type Program struct {
	Name        string
	Macros      []*macro.Macro
	Items       map[string]items.Item
	Coordinates map[string]*coordinates.Coordinates
}

var programs = make(Programs)

func ReadPrograms() *Programs { return &programs }
func (ps *Programs) CreateProgram(name string) {
	p := &Program{
		Name:   name,
		Macros: []*macro.Macro{},
		Items:  make(map[string]items.Item), //make(map[string]items.Item),
		Coordinates: map[string]*coordinates.Coordinates{
			strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
				Points:      make(map[string]coordinates.Point),
				SearchAreas: make(map[string]coordinates.SearchArea),
			},
		},
	}
	programs[name] = p
}
func (ps *Programs) ReadProgram(name string) *Program {
	return programs[name]
}

func (ps *Programs) UpdateProgram(name string) {
	_, ok := programs[name]
	if ok {

	}
}

func (ps *Programs) DeleteProgram(name string) {
	val, ok := programs[name]
	if ok {
		programs[val.Name] = nil
	}
}

func (ps *Programs) ReadAllMacros() []*macro.Macro {
	ms := []*macro.Macro{}
	for _, p := range *ps {
		ms = append(ms, p.Macros...)
	}
	return ms
}
func (p *Program) ReadMacroAtIndex(i int) *macro.Macro { return p.Macros[i] }
func (p *Program) ReadMacroByName(s string) *macro.Macro {
	for _, m := range p.Macros {
		if m.Name == s {
			return m
		}
	}
	return nil
}

func (p *Program) CreateMacro(s string, d int) {
	if s == "" {
		return
	}
	p.Macros = append(p.Macros, macro.CreateMacro(s, d, []string{}))
}

func (ps *Programs) ReadAllCoordinates() []*coordinates.Coordinates {
	cs := []*coordinates.Coordinates{}
	for _, p := range *ps {
		cs = append(cs, p.Coordinates[config.MainMonitorSizeString])
	}
	return cs
}

func (ps *Programs) ReadAllPoints() []*coordinates.Point {
	allPoints := []*coordinates.Point{}
	for _, pro := range *ps {
		for _, cs := range pro.Coordinates {
			for _, poi := range cs.Points {
				allPoints = append(allPoints, &poi)

			}
		}
	}
	return allPoints
}

func (ps *Programs) ReadAllPointsAsStringSlice() []string {
	allPoints := ps.ReadAllPoints
	allPointsStringSlice := []string{}
	for _, poi := range allPoints() {
		allPointsStringSlice = append(allPointsStringSlice, poi.Name)
	}
	return allPointsStringSlice
}

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
