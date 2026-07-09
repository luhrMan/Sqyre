package repositories

import (
	"Sqyre/internal/models"
)

func init() {
	models.CollectionRepositoryFactory = func(p *models.Program) models.CollectionRepositoryInterface {
		return NewCollectionRepository(p)
	}
}

// CollectionRepository manages Collection persistence within a Program context.
type CollectionRepository struct {
	*NestedRepository[models.Collection]
	program *models.Program
}

// NewCollectionRepository creates a CollectionRepository for a Program.
func NewCollectionRepository(program *models.Program) *CollectionRepository {
	if program.Collections == nil {
		program.Collections = make(map[string]*models.Collection)
	}

	return &CollectionRepository{
		NestedRepository: NewNestedRepository(
			program.Collections,
			program.GetKey(),
			func() error {
				return ProgramRepo().Set(program.GetKey(), program)
			},
		),
		program: program,
	}
}

// New creates a new Collection instance with default values.
func (r *CollectionRepository) New() *models.Collection {
	return &models.Collection{
		Name:       "",
		SearchArea: "",
		Rows:       1,
		Cols:       1,
	}
}
