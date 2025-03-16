package internal

import (
	"Squire/encoding"
	"Squire/internal/data"
	"log"
)

var p *Programs

type Programs struct {
	Map *map[string]Program
}

type Program struct {
	Macros      *[]Macro
	Items       *map[string]data.Item
	Coordinates *map[[2]int]ScreenSize
}

type ScreenSize struct {
	Points      *map[string]data.Point
	SearchAreas *map[string]data.SearchArea
}

func GetPrograms() *Programs {
	// var err error
	if p == nil {
		p = &Programs{}
		a, err := encoding.GobSerializer.Decode("programData")
		if err != nil {
			log.Println(err)
			return nil
		}
		if a == nil {
			return p
		}
		p = a.(*Programs)
		return p
	}
	return p
}

func (p *Program) AddProgramPoint(screenSize [2]int, point data.Point) {
	c := *p.Coordinates
	points := *c[screenSize].Points
	points[point.Name] = point
}

func (p *Program) AddProgramSearchArea(screenSize [2]int, sa data.SearchArea) {
	c := *p.Coordinates
	sas := *c[screenSize].SearchAreas
	sas[sa.Name] = sa
}
