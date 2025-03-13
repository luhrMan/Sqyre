package internal

import "Squire/internal/data"

type Program struct {
	Macros      *[]Macro
	Items       *map[string]data.Item
	Coordinates *map[[2]int]ScreenSize
}

type ScreenSize struct {
	Points      *map[string]data.Point
	SearchAreas *map[string]data.SearchArea
}
