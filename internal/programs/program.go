package programs

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"Squire/internal/programs/macro"
	"strconv"
)

type Program struct {
	Macros      []*macro.Macro
	Items       map[string]items.Item
	Coordinates map[string]*coordinates.Coordinates
}

func NewProgram() *Program {
	return &Program{
		Macros: []*macro.Macro{},
		Items:  make(map[string]items.Item),
		Coordinates: map[string]*coordinates.Coordinates{
			"2560x1440": {
				Points:      make(map[string]coordinates.Point),
				SearchAreas: make(map[string]coordinates.SearchArea),
			},
			"1920x1080": {
				Points: map[string]coordinates.Point{
					// "test": {
					// 	Name: "test",
					// 	X:    10,
					// 	Y:    10,
					// },
				},
				SearchAreas: map[string]coordinates.SearchArea{
					// "test": {
					// 	Name:    "test",
					// 	LeftX:   10,
					// 	TopY:    10,
					// 	RightX:  10,
					// 	BottomY: 10,
					// },
				},
			},
		},
	}
}
func CurrentProgramAndScreenSizeCoordinates() *coordinates.Coordinates {
	// log.Println(strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight))
	// log.Println(currentProgram.Coordinates[strconv.Itoa(config.MonitorWidth)+"x"+strconv.Itoa(config.MonitorHeight)])
	return currentProgram.Coordinates[strconv.Itoa(config.MonitorWidth)+"x"+strconv.Itoa(config.MonitorHeight)]
}

func (p *Program) GetMacroAtIndex(i int) *macro.Macro { return p.Macros[i] }
func (p *Program) GetMacroByName(s string) *macro.Macro {
	for _, m := range p.Macros {
		if m.Name == s {
			return m
		}
	}
	return nil
}

func (p *Program) AddMacro(s string, d int) {
	if s == "" {
		return
	}
	p.Macros = append(p.Macros, macro.NewMacro(s, d, []string{}))
}

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
