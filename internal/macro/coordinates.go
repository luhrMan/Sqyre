package macro

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
)

// LookupSearchArea resolves a search area reference from the program repository.
// Collection refs are not valid here; use ResolveSearchAreaCoordsFromRef instead.
func LookupSearchArea(ref actions.CoordinateRef, resolutionKey string) (*models.SearchArea, error) {
	if ref.IsCollection() {
		return nil, fmt.Errorf("search area lookup does not accept collection ref %q", ref)
	}
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
// Collection refs are not valid here; use ResolvePointCoordsFromRef instead.
func LookupPoint(ref actions.CoordinateRef, resolutionKey string) (*models.Point, error) {
	if ref.IsCollection() {
		return nil, fmt.Errorf("point lookup does not accept collection ref %q", ref)
	}
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

// LookupCollection resolves a collection by program and name (cell-range suffix ignored for lookup).
func LookupCollection(ref actions.CoordinateRef) (*models.Collection, error) {
	name := ref.Name()
	if name == "" {
		return nil, fmt.Errorf("empty collection reference")
	}
	if programName := ref.Program(); programName != "" {
		return collectionFromProgram(programName, name)
	}
	for _, programName := range repositories.ProgramRepo().GetAllKeys() {
		c, err := collectionFromProgram(programName, name)
		if err == nil {
			return c, nil
		}
	}
	return nil, fmt.Errorf("collection %q not found", ref)
}

// ResolveSearchAreaCoordsFromRef looks up a search area or collection selection and resolves its rectangle.
func ResolveSearchAreaCoordsFromRef(ref actions.CoordinateRef, macro *models.Macro, resolutionKey string) (int, int, int, int, error) {
	if ref.IsCollection() {
		return resolveCollectionRect(ref, macro, resolutionKey)
	}
	sa, err := LookupSearchArea(ref, resolutionKey)
	if err != nil {
		return 0, 0, 0, 0, err
	}
	return ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
}

// ResolvePointCoordsFromRef resolves a point or collection selection to screen coordinates.
func ResolvePointCoordsFromRef(ref actions.CoordinateRef, macro *models.Macro, resolutionKey string) (x, y int, err error) {
	if ref.IsCollection() {
		lx, ty, rx, by, err := resolveCollectionRect(ref, macro, resolutionKey)
		if err != nil {
			return 0, 0, err
		}
		return (lx + rx) / 2, (ty + by) / 2, nil
	}
	pt, err := LookupPoint(ref, resolutionKey)
	if err != nil {
		return 0, 0, err
	}
	x, err = ResolveInt(pt.X, macro)
	if err != nil {
		return 0, 0, fmt.Errorf("point X: %w", err)
	}
	y, err = ResolveInt(pt.Y, macro)
	if err != nil {
		return 0, 0, fmt.Errorf("point Y: %w", err)
	}
	return x, y, nil
}

func resolveCollectionRect(ref actions.CoordinateRef, macro *models.Macro, resolutionKey string) (int, int, int, int, error) {
	r1, c1, r2, c2, ok := ref.CellRange()
	if !ok {
		return 0, 0, 0, 0, fmt.Errorf("invalid collection cell range in %q", ref)
	}
	col, err := LookupCollection(ref)
	if err != nil {
		return 0, 0, 0, 0, err
	}
	programName := ref.Program()
	if programName == "" {
		// LookupCollection already found it; re-resolve program for search area.
		for _, pn := range repositories.ProgramRepo().GetAllKeys() {
			if c, e := collectionFromProgram(pn, col.Name); e == nil && c != nil {
				programName = pn
				break
			}
		}
	}
	if programName == "" {
		return 0, 0, 0, 0, fmt.Errorf("collection %q: could not determine program", ref)
	}
	sa, err := searchAreaFromProgram(programName, col.SearchArea, resolutionKey)
	if err != nil {
		return 0, 0, 0, 0, fmt.Errorf("collection %q search area %q: %w", col.Name, col.SearchArea, err)
	}
	lx, ty, rx, by, err := ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
	if err != nil {
		return 0, 0, 0, 0, err
	}
	return models.CellRect(lx, ty, rx, by, col.Rows, col.Cols, models.CellRange{R1: r1, C1: c1, R2: r2, C2: c2})
}

func searchAreaFromProgram(programName, name, resolutionKey string) (*models.SearchArea, error) {
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		return nil, err
	}
	saRepo, err := program.SearchAreaRepo(resolutionKey)
	if err != nil {
		return nil, err
	}
	sa, err := saRepo.Get(name)
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
	ptRepo, err := program.PointRepo(resolutionKey)
	if err != nil {
		return nil, err
	}
	pt, err := ptRepo.Get(name)
	if err != nil {
		return nil, err
	}
	return pt, nil
}

func collectionFromProgram(programName, name string) (*models.Collection, error) {
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		return nil, err
	}
	repo, err := program.CollectionRepo()
	if err != nil {
		return nil, err
	}
	c, err := repo.Get(name)
	if err != nil {
		return nil, err
	}
	return c, nil
}

// DefaultResolutionKey returns the resolution key used for coordinate lookups at runtime.
func DefaultResolutionKey() string {
	return config.MainMonitorSizeString
}
