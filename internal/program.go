package internal

import (
	"Squire/internal/data"
	"log"
	"strconv"
)

var programs = &Programs{}

type Programs map[string]*Program

type Program struct {
	Macros      []*Macro
	Items       map[string]data.Item
	Coordinates map[string]data.Coordinates
}

func NewProgram() *Program {
	return &Program{
		Macros: []*Macro{},
		Items:  make(map[string]data.Item),
		Coordinates: map[string]data.Coordinates{
			"2560x1440": {
				Points:      make(map[string]data.Point),
				SearchAreas: make(map[string]data.SearchArea),
			},
		},
	}
}

func GetPrograms() *Programs                        { return programs }
func (p *Programs) GetProgram(name string) *Program { return (*p)[name] }
func (p *Program) GetMacroAtIndex(i int) *Macro     { return p.Macros[i] }
func (p *Program) GetMacroByName(s string) *Macro {
	for _, m := range p.Macros {
		if m.Name == s {
			return m
		}
	}
	return nil
}

func (p *Programs) InitPrograms() {
	(*p)[data.DarkAndDarker] = NewProgram()
	macros := data.ViperConfig.GetStringSlice("programs" + "." + data.DarkAndDarker + "." + "macros")
	for i := range macros {
		p.GetProgram(data.DarkAndDarker).Macros = append(p.GetProgram(data.DarkAndDarker).Macros, NewMacro("New Macro "+strconv.Itoa(i), 30, ""))
		err := p.GetProgram(data.DarkAndDarker).GetMacroAtIndex(i).UnmarshalMacro(i)
		if err != nil {
			log.Println(err)
		}
	}
}

func (p *Program) AddProgramPoint(ss string, point data.Point) {
	c := p.Coordinates
	points := c[ss].Points
	points[point.Name] = point
}
func (p *Program) SerializeJsonPointsToProgram(ss string) {
	jpm := data.JsonPointMap()
	for _, point := range jpm {
		p.AddProgramPoint(ss, point)
	}
}

func (p *Program) AddProgramSearchArea(ss string, sa data.SearchArea) {
	c := p.Coordinates
	sas := c[ss].SearchAreas
	sas[sa.Name] = sa
}
