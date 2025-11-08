package repositories

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"fmt"
	"log"
	"maps"
	"slices"
	"strings"
	"sync"
)

func init() {
	// Set the factory function to avoid circular dependency
	models.ItemRepositoryFactory = func(p *models.Program) models.ItemRepositoryInterface {
		return NewItemRepository(p)
	}
}

// ItemRepository manages Item persistence within a Program context.
// Unlike BaseRepository, ItemRepository is scoped to a specific Program
// and operates on a reference to the Program's Items map.
type ItemRepository struct {
	mu          sync.RWMutex
	items       map[string]*models.Item // Reference to Program.Items
	programName string                  // For logging and error context
	program     *models.Program         // Parent aggregate for saves
}

// NewItemRepository creates an ItemRepository for a Program.
// The repository operates on a reference to the Program's Items map,
// not a copy, ensuring single source of truth.
func NewItemRepository(program *models.Program) *ItemRepository {
	return &ItemRepository{
		items:       program.Items,
		programName: program.Name,
		program:     program,
	}
}

// Get retrieves an item by name (case-insensitive).
// Returns ErrNotFound if the item doesn't exist.
func (r *ItemRepository) Get(name string) (*models.Item, error) {
	if name == "" {
		return nil, ErrInvalidKey
	}

	r.mu.RLock()
	defer r.mu.RUnlock()

	normalizedName := strings.ToLower(name)
	item, ok := r.items[normalizedName]
	if !ok {
		return nil, fmt.Errorf("%w: item '%s' in program '%s'", ErrNotFound, name, r.programName)
	}

	return item, nil
}

// GetAll returns a copy of all items.
// This is safe for concurrent access and prevents external modification.
func (r *ItemRepository) GetAll() map[string]*models.Item {
	r.mu.RLock()
	defer r.mu.RUnlock()

	// Return a copy to prevent external modification
	result := make(map[string]*models.Item, len(r.items))
	maps.Copy(result, r.items)

	return result
}

// GetAllKeys returns a sorted slice of all item names
func (r *ItemRepository) GetAllKeys() []string {
	r.mu.RLock()
	defer r.mu.RUnlock()

	keys := make([]string, 0, len(r.items))
	for k := range r.items {
		keys = append(keys, k)
	}

	slices.Sort(keys)
	return keys
}

// Set creates or updates an item.
// Names are normalized to lowercase. Changes are immediately persisted.
func (r *ItemRepository) Set(name string, item *models.Item) error {
	if name == "" {
		return ErrInvalidKey
	}
	if item == nil {
		return fmt.Errorf("item cannot be nil")
	}

	r.mu.Lock()
	normalizedName := strings.ToLower(name)
	r.items[normalizedName] = item
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist item '%s' in program '%s': %w", name, r.programName, err)
	}

	log.Printf("Set item '%s' in program '%s'", name, r.programName)
	return nil
}

// Delete removes an item by name. This operation is idempotent.
// Changes are immediately persisted.
func (r *ItemRepository) Delete(name string) error {
	if name == "" {
		return ErrInvalidKey
	}

	r.mu.Lock()
	normalizedName := strings.ToLower(name)
	delete(r.items, normalizedName)
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist after deleting item '%s' in program '%s': %w", name, r.programName, err)
	}

	log.Printf("Deleted item '%s' from program '%s'", name, r.programName)
	return nil
}

// Save persists changes by saving the parent Program through ProgramRepository.
// Since Items are part of the Program aggregate, we save the entire Program.
func (r *ItemRepository) Save() error {
	// Save the entire program (which includes all items)
	if err := ProgramRepo().Set(r.programName, r.program); err != nil {
		return fmt.Errorf("%w: failed to save program '%s': %v", ErrSaveFailed, r.programName, err)
	}

	return nil
}

// Count returns the number of items in the repository
func (r *ItemRepository) Count() int {
	r.mu.RLock()
	defer r.mu.RUnlock()

	return len(r.items)
}

// GetAllWithProgramPrefix returns all items with keys formatted as "program|item".
// This is useful for UI components that need to display items with their program context.
func (r *ItemRepository) GetAllWithProgramPrefix() map[string]*models.Item {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make(map[string]*models.Item, len(r.items))
	for itemName, item := range r.items {
		prefixedKey := r.programName + config.ProgramDelimiter + itemName
		result[prefixedKey] = item
	}

	return result
}

// GetAllSorted returns all item names in alphabetical order.
// This is useful for UI components that need to display items in a sorted list.
func (r *ItemRepository) GetAllSorted() []string {
	r.mu.RLock()
	defer r.mu.RUnlock()

	keys := make([]string, 0, len(r.items))
	for k := range r.items {
		keys = append(keys, k)
	}

	slices.Sort(keys)
	return keys
}
