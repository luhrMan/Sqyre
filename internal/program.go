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
	Macros      []Macro
	Items       map[string]data.Item
	Coordinates map[ScreenSize]data.Coordinates
}
type ScreenSize [2]int

func (p *Programs) NewProgram(name string) {
	(*p)[name] = &Program{
		Macros: []Macro{
			NewMacro("New Macro", 30, ""),
			NewMacro("Macro 2", 30, ""),
		},
		Items: make(map[string]data.Item),
		Coordinates: map[ScreenSize]data.Coordinates{
			{2560, 1440}: {
				Points:      make(map[string]data.Point),
				SearchAreas: make(map[string]data.SearchArea),
			},
		},
	}
}

func GetPrograms() *Programs {
	if p != nil {
		log.Println("p already has a value:", p)
		return &p
	}
	log.Println("couldn't load programs, create fresh set")
	p = make(Programs)
	p.InitPrograms()
	return &p
}

func (p *Programs) GetProgram(name string) *Program {
	return (*p)[name]
}

func (p *Programs) InitPrograms() {
	err := encoding.GobSerializer.Decode("programData", p)
	if err != nil {
		log.Println(err)
		p.NewProgram(data.DarkAndDarker)
		p.GetProgram(data.DarkAndDarker).SerializeJsonPointsToProgram(ScreenSize{2560, 1440})
	}
}
func (p Program) GetMacroAtIndex(i int) Macro {
	return p.Macros[i]
}

// func DecodePrograms() Programs {
// 	p = Programs{}
// 	err := encoding.GobSerializer.Decode("programData", &p)
// 	if err != nil {
// 		log.Println(err)
// 		return nil
// 	}
// 	return p
// }

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
