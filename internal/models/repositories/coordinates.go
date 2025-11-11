package repositories

import (
	"Squire/internal/models"
)

func init() {
	// Set the factory functions to avoid circular dependency
	models.PointRepositoryFactory = func(p *models.Program, resKey string) models.PointRepositoryInterface {
		return NewPointRepository(p, resKey)
	}
	
	models.SearchAreaRepositoryFactory = func(p *models.Program, resKey string) models.SearchAreaRepositoryInterface {
		return NewSearchAreaRepository(p, resKey)
	}
}

// PointRepository manages Point persistence within a Program context.
// It embeds NestedRepository to leverage generic CRUD operations while maintaining
// program and resolution context for specialized behavior.
type PointRepository struct {
	*NestedRepository[models.Point]
	resolutionKey string          // e.g., "2560x1440"
	program       *models.Program // Parent aggregate for context
}

// NewPointRepository creates a PointRepository for a Program at a specific resolution.
// The repository operates on a reference to the Program's Coordinates.Points map,
// not a copy, ensuring single source of truth.
// If the Coordinates entry for the resolution doesn't exist, it will be created.
func NewPointRepository(program *models.Program, resolutionKey string) *PointRepository {
	// Initialize Coordinates map entry if it doesn't exist
	coords := program.Coordinates[resolutionKey]
	if coords == nil {
		coords = &models.Coordinates{
			Points:      make(map[string]*models.Point),
			SearchAreas: make(map[string]*models.SearchArea),
		}
		program.Coordinates[resolutionKey] = coords
	}
	
	return &PointRepository{
		NestedRepository: NewNestedRepository[models.Point](
			coords.Points,
			program.GetKey()+"|"+resolutionKey,
			func() error {
				return ProgramRepo().Set(program.GetKey(), program)
			},
		),
		resolutionKey: resolutionKey,
		program:       program,
	}
}


// SearchAreaRepository manages SearchArea persistence within a Program context.
// It embeds NestedRepository to leverage generic CRUD operations while maintaining
// program and resolution context for specialized behavior.
type SearchAreaRepository struct {
	*NestedRepository[models.SearchArea]
	resolutionKey string          // e.g., "2560x1440"
	program       *models.Program // Parent aggregate for context
}

// NewSearchAreaRepository creates a SearchAreaRepository for a Program at a specific resolution.
// The repository operates on a reference to the Program's Coordinates.SearchAreas map,
// not a copy, ensuring single source of truth.
// If the Coordinates entry for the resolution doesn't exist, it will be created.
func NewSearchAreaRepository(program *models.Program, resolutionKey string) *SearchAreaRepository {
	// Initialize Coordinates map entry if it doesn't exist
	coords := program.Coordinates[resolutionKey]
	if coords == nil {
		coords = &models.Coordinates{
			Points:      make(map[string]*models.Point),
			SearchAreas: make(map[string]*models.SearchArea),
		}
		program.Coordinates[resolutionKey] = coords
	}
	
	return &SearchAreaRepository{
		NestedRepository: NewNestedRepository[models.SearchArea](
			coords.SearchAreas,
			program.GetKey()+"|"+resolutionKey,
			func() error {
				return ProgramRepo().Set(program.GetKey(), program)
			},
		),
		resolutionKey: resolutionKey,
		program:       program,
	}
}


