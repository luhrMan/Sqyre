package internal

import (
	"Squire/encoding"
	"Squire/internal/data"
	"log"
)

var p Programs

// type Name string
type Programs map[string]*Program

type Program struct {
	Macros      *[]Macro
	Items       map[string]data.Item
	Coordinates map[ScreenSize]data.Coordinates
}
type ScreenSize [2]int

func NewProgram() *Program {
	return &Program{
		Macros: &[]Macro{
			*NewMacro("test", 30, ""),
		},
		Items: map[string]data.Item{},
		Coordinates: map[ScreenSize]data.Coordinates{
			{2560, 1440}: {
				Points:      map[string]data.Point{},
				SearchAreas: map[string]data.SearchArea{},
			},
		},
	}
}

func GetPrograms() Programs {
	// var err error
	if p == nil {
		p = Programs{}
		a, err := encoding.GobSerializer.Decode("programData")
		if err != nil {
			log.Println(err)
			return p
		}
		if a == nil {
			return p
		}
		p = a.(Programs)
		return p
	}
	return p
}

// func (p *Program) GetMacros() *[]Macro {
// 	return p.Macros
// }

func (p *Program) AddProgramPoint(ss ScreenSize, point data.Point) {
	c := p.Coordinates
	points := c[ss].Points
	points[point.Name] = point
}
func (p *Program) SerializeJsonPointsToProgram(ss ScreenSize) {
	jpm := data.JsonPointMap()
	for _, point := range jpm {
		p.AddProgramPoint(ss, point)
	}
}

func (p *Program) AddProgramSearchArea(ss ScreenSize, sa data.SearchArea) {
	c := p.Coordinates
	sas := c[ss].SearchAreas
	sas[sa.Name] = sa
}
