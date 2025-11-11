package repositories

import (
	"Squire/internal/config"
	"Squire/internal/models"
)

func init() {
	// Set the factory function to avoid circular dependency
	models.ItemRepositoryFactory = func(p *models.Program) models.ItemRepositoryInterface {
		return NewItemRepository(p)
	}
}

// ItemRepository manages Item persistence within a Program context.
// It embeds NestedRepository to leverage generic CRUD operations while
// providing custom methods specific to Item management.
type ItemRepository struct {
	*NestedRepository[models.Item]
	program *models.Program // Parent aggregate for custom methods
}

// NewItemRepository creates an ItemRepository for a Program.
// The repository operates on a reference to the Program's Items map,
// not a copy, ensuring single source of truth.
func NewItemRepository(program *models.Program) *ItemRepository {
	return &ItemRepository{
		NestedRepository: NewNestedRepository[models.Item](
			program.Items,
			program.GetKey(),
			func() error {
				return ProgramRepo().Set(program.GetKey(), program)
			},
		),
		program: program,
	}
}

// GetAllWithProgramPrefix returns all items with keys formatted as "program|item".
// This is useful for UI components that need to display items with their program context.
func (r *ItemRepository) GetAllWithProgramPrefix() map[string]*models.Item {
	items := r.GetAll() // Use embedded NestedRepository method

	result := make(map[string]*models.Item, len(items))
	for itemName, item := range items {
		prefixedKey := r.program.GetKey() + config.ProgramDelimiter + itemName
		result[prefixedKey] = item
	}

	return result
}

// GetAllSorted returns all item names in alphabetical order.
// This is useful for UI components that need to display items in a sorted list.
func (r *ItemRepository) GetAllSorted() []string {
	return r.GetAllKeys() // Use embedded NestedRepository method
}
