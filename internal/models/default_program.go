package models

import "Sqyre/internal/config"

// DefaultProgramName is the built-in program that exposes screen-relative search areas
// using ${screenMinX}, ${screenMidX}, etc. (populated each macro run by services.ApplyScreenBoundsVariables).
const DefaultProgramName = "default"

// SeedDefaultScreenSearchAreas adds standard regions for the given resolution key.
// Coordinates use macro variables so they track the virtual desktop at execution time.
func SeedDefaultScreenSearchAreas(p *Program, resolutionKey string) {
	coords := p.Coordinates[resolutionKey]
	if coords == nil {
		coords = &Coordinates{
			Points:      make(map[string]*Point),
			SearchAreas: make(map[string]*SearchArea),
		}
		p.Coordinates[resolutionKey] = coords
	}
	areas := map[string]*SearchArea{
		"whole screen": {
			Name:    "whole screen",
			LeftX:   "${screenMinX}",
			TopY:    "${screenMinY}",
			RightX:  "${screenMaxX}",
			BottomY: "${screenMaxY}",
		},
		"left half": {
			Name:    "left half",
			LeftX:   "${screenMinX}",
			TopY:    "${screenMinY}",
			RightX:  "${screenMidX}",
			BottomY: "${screenMaxY}",
		},
		"right half": {
			Name:    "right half",
			LeftX:   "${screenMidX}",
			TopY:    "${screenMinY}",
			RightX:  "${screenMaxX}",
			BottomY: "${screenMaxY}",
		},
		"top half": {
			Name:    "top half",
			LeftX:   "${screenMinX}",
			TopY:    "${screenMinY}",
			RightX:  "${screenMaxX}",
			BottomY: "${screenMidY}",
		},
		"bottom half": {
			Name:    "bottom half",
			LeftX:   "${screenMinX}",
			TopY:    "${screenMidY}",
			RightX:  "${screenMaxX}",
			BottomY: "${screenMaxY}",
		},
	}
	for name, sa := range areas {
		coords.SearchAreas[name] = sa
	}
}

// NewDefaultProgram returns a program named DefaultProgramName with default screen search areas
// for the current main monitor size string.
func NewDefaultProgram() *Program {
	p := NewProgram()
	p.Name = DefaultProgramName
	SeedDefaultScreenSearchAreas(p, config.MainMonitorSizeString)
	return p
}
