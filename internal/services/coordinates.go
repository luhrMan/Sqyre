package services

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
)

// LookupSearchArea resolves a search area reference from the program repository.
func LookupSearchArea(ref actions.CoordinateRef, resolutionKey string) (*models.SearchArea, error) {
	name := ref.Name()
	if name == "" {
		return nil, fmt.Errorf("empty search area reference")
	}
	if programName := ref.Program(); programName != "" {
		return searchAreaFromProgram(programName, name, resolutionKey)
	}
	for _, programName := range repositories.ProgramRepo().GetAllKeys() {
		sa, err := searchAreaFromProgram(programName, name, resolutionKey)
		if err == nil {
			return sa, nil
		}
	}
	return nil, fmt.Errorf("search area %q not found", ref)
}

// LookupPoint resolves a point reference from the program repository.
func LookupPoint(ref actions.CoordinateRef, resolutionKey string) (*models.Point, error) {
	name := ref.Name()
	if name == "" {
		return nil, fmt.Errorf("empty point reference")
	}
	if programName := ref.Program(); programName != "" {
		return pointFromProgram(programName, name, resolutionKey)
	}
	for _, programName := range repositories.ProgramRepo().GetAllKeys() {
		pt, err := pointFromProgram(programName, name, resolutionKey)
		if err == nil {
			return pt, nil
		}
	}
	return nil, fmt.Errorf("point %q not found", ref)
}

// ResolveSearchAreaCoordsFromRef looks up a search area and resolves its coordinates.
func ResolveSearchAreaCoordsFromRef(ref actions.CoordinateRef, macro *models.Macro, resolutionKey string) (int, int, int, int, error) {
	sa, err := LookupSearchArea(ref, resolutionKey)
	if err != nil {
		return 0, 0, 0, 0, err
	}
	return ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
}

func searchAreaFromProgram(programName, name, resolutionKey string) (*models.SearchArea, error) {
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		return nil, err
	}
	sa, err := program.SearchAreaRepo(resolutionKey).Get(name)
	if err != nil {
		return nil, err
	}
	return sa, nil
}

func pointFromProgram(programName, name, resolutionKey string) (*models.Point, error) {
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		return nil, err
	}
	pt, err := program.PointRepo(resolutionKey).Get(name)
	if err != nil {
		return nil, err
	}
	return pt, nil
}

// DefaultResolutionKey returns the resolution key used for coordinate lookups at runtime.
func DefaultResolutionKey() string {
	return config.MainMonitorSizeString
}
