package repositories

import (
	"fmt"
	"maps"
	"slices"
	"strings"
	"sync"
)

// NestedRepository manages models that exist within an aggregate root context.
// Unlike BaseRepository which persists directly to config via Viper, NestedRepository
// saves changes by persisting the parent aggregate root (e.g., Program).
//
// # Purpose
//
// NestedRepository implements the Aggregate Root pattern from Domain-Driven Design,
// ensuring that child entities are always persisted as part of their parent aggregate.
// This maintains data consistency and enforces proper boundaries between aggregates.
//
// # Aggregate Root Pattern
//
// An aggregate root is a domain model that owns and manages child entities:
// - The root is the entry point for all operations on the aggregate
// - Child entities cannot be modified independently
// - Changes to children trigger persistence of the entire aggregate
// - The root maintains consistency across all its children
//
// In Squire:
// - Program is an aggregate root that owns Items, Points, and SearchAreas
// - When an Item changes, we save the entire Program
// - This ensures Items are always consistent with their Program
//
// # When to Use NestedRepository
//
// Use NestedRepository for:
// - Models that belong to an aggregate root (Item, Point, SearchArea)
// - Models that should be saved as part of their parent
// - Models that don't exist independently in the config
// - Models that need context from their parent
//
// Example config structure for NestedRepository models:
//
//	programs:
//	  "dark and darker":
//	    name: "dark and darker"
//	    items:                          # Managed by ItemRepository
//	      "health-potion":
//	        name: "health-potion"
//	        width: 1
//	        height: 2
//	    coordinates:
//	      "1920x1080":
//	        points:                     # Managed by PointRepository
//	          "stash-button":
//	            name: "stash-button"
//	            x: 100
//	            y: 200
//	        searchAreas:                # Managed by SearchAreaRepository
//	          "inventory":
//	            name: "inventory"
//	            x: 50
//	            y: 50
//
// # When NOT to Use NestedRepository
//
// Do NOT use NestedRepository for:
// - Top-level models (Macro, Program) - use BaseRepository instead
// - Models that need independent persistence
// - Models that don't belong to an aggregate root
//
// See BaseRepository for managing top-level models.
//
// # Architecture: BaseRepository vs NestedRepository
//
// BaseRepository (for top-level models):
//   - Persists directly to config via Viper.WriteConfig()
//   - Each model is independent
//   - Example: Macro, Program
//   - Config structure: macros.{name}, programs.{name}
//
// NestedRepository (for child models):
//   - Persists by saving the parent aggregate root
//   - Models are part of a larger aggregate
//   - Example: Item (within Program), Point (within Program/Coordinates)
//   - Config structure: programs.{name}.items.{itemName}
//
// # Key Differences from BaseRepository
//
// 1. Persistence:
//    - BaseRepository: Calls Viper.WriteConfig() directly
//    - NestedRepository: Calls saveFunc to persist parent aggregate
//
// 2. Initialization:
//    - BaseRepository: Loads from config key (e.g., "macros")
//    - NestedRepository: Operates on parent's existing map reference
//
// 3. Context:
//    - BaseRepository: No parent context needed
//    - NestedRepository: Requires contextKey for error messages (e.g., "dark and darker")
//
// 4. Lifecycle:
//    - BaseRepository: Models exist independently
//    - NestedRepository: Models lifecycle tied to parent aggregate
//
// # Type Parameters
//
// T: The model type. While T can be any type, *T should implement BaseModel interface
// to work correctly with repository operations. The repository stores pointers to T (*T).
//
// # Thread Safety
//
// All operations are thread-safe using sync.RWMutex:
// - Read operations (Get, GetAll, GetAllKeys, Count) use RLock
// - Write operations (Set, Delete) use Lock
// - Multiple concurrent reads are allowed
// - Writes are exclusive
//
// # Example Usage
//
// Creating a repository for Items within a Program:
//
//	itemRepo := NewNestedRepository[models.Item](
//	    program.Items,                    // Reference to parent's map
//	    program.GetKey(),                 // Context: "dark and darker"
//	    func() error {                    // Save the parent Program
//	        return ProgramRepo().Set(program.GetKey(), program)
//	    },
//	)
//
// Creating a repository for Points within a Program/Resolution:
//
//	coords := program.Coordinates["1920x1080"]
//	pointRepo := NewNestedRepository[models.Point](
//	    coords.Points,                    // Reference to parent's map
//	    program.GetKey() + "|1920x1080",  // Context: "dark and darker|1920x1080"
//	    func() error {                    // Save the parent Program
//	        return ProgramRepo().Set(program.GetKey(), program)
//	    },
//	)
//
// Using the repository (same API as BaseRepository):
//
//	// Create and store a model
//	item := &models.Item{Name: "health-potion"}
//	err := itemRepo.Set("health-potion", item)  // Saves parent Program
//
//	// Retrieve a model
//	retrieved, err := itemRepo.Get("health-potion")
//
//	// List all models
//	allItems := itemRepo.GetAll()               // Returns a copy
//	keys := itemRepo.GetAllKeys()               // Sorted keys
//
//	// Delete a model
//	err = itemRepo.Delete("health-potion")      // Saves parent Program
//
// # Benefits of Aggregate Root Pattern
//
// 1. Consistency: All child entities are saved atomically with their parent
// 2. Encapsulation: Parent controls access to children
// 3. Simplicity: No need to track which children have changed
// 4. Performance: Single write operation for multiple child changes
// 5. Correctness: Impossible to have orphaned child entities
type NestedRepository[T any] struct {
	mu         sync.RWMutex
	models     map[string]*T // Reference to parent's model map
	contextKey string        // Identifier for logging/errors (e.g., program name)
	saveFunc   func() error  // Function to persist the parent aggregate
}

// NewNestedRepository creates a repository for models within an aggregate root.
//
// Parameters:
//   - models: reference to the parent's model map (e.g., program.Items)
//   - contextKey: identifier for logging/errors (e.g., "dark and darker" or "dark and darker|1920x1080")
//   - saveFunc: function that persists the parent aggregate (e.g., save Program)
//
// The models map is not copied - the repository operates directly on the parent's map.
// This ensures changes are immediately visible to the parent and other code.
func NewNestedRepository[T any](
	models map[string]*T,
	contextKey string,
	saveFunc func() error,
) *NestedRepository[T] {
	return &NestedRepository[T]{
		models:     models,
		contextKey: contextKey,
		saveFunc:   saveFunc,
	}
}

// Get retrieves a model by key. Returns ErrNotFound if the key doesn't exist.
// Keys are normalized to lowercase for case-insensitive access.
func (r *NestedRepository[T]) Get(key string) (*T, error) {
	if key == "" {
		return nil, ErrInvalidKey
	}

	r.mu.RLock()
	defer r.mu.RUnlock()

	normalizedKey := strings.ToLower(key)
	model, ok := r.models[normalizedKey]
	if !ok {
		return nil, fmt.Errorf("%w: %s (context: %s)", ErrNotFound, key, r.contextKey)
	}

	return model, nil
}

// GetAll returns a copy of all models in the repository.
// This is safe for concurrent access and prevents external modification of the internal map.
func (r *NestedRepository[T]) GetAll() map[string]*T {
	r.mu.RLock()
	defer r.mu.RUnlock()

	// Return a copy to prevent external modification
	result := make(map[string]*T, len(r.models))
	maps.Copy(result, r.models)

	return result
}

// GetAllKeys returns a sorted slice of all keys in the repository.
func (r *NestedRepository[T]) GetAllKeys() []string {
	r.mu.RLock()
	defer r.mu.RUnlock()

	keys := make([]string, 0, len(r.models))
	for k := range r.models {
		keys = append(keys, k)
	}

	slices.Sort(keys)
	return keys
}

// Set creates or updates a model with the given key.
// Keys are normalized to lowercase. Changes are immediately persisted via saveFunc.
func (r *NestedRepository[T]) Set(key string, model *T) error {
	if key == "" {
		return ErrInvalidKey
	}
	if model == nil {
		return fmt.Errorf("model cannot be nil")
	}

	r.mu.Lock()
	normalizedKey := strings.ToLower(key)
	r.models[normalizedKey] = model
	r.mu.Unlock()

	// Save the parent aggregate immediately after modification
	if err := r.saveFunc(); err != nil {
		return fmt.Errorf("failed to persist aggregate after set (context: %s): %w", r.contextKey, err)
	}

	return nil
}

// Delete removes a model by key. This operation is idempotent.
// Changes are immediately persisted via saveFunc.
func (r *NestedRepository[T]) Delete(key string) error {
	if key == "" {
		return ErrInvalidKey
	}

	r.mu.Lock()
	normalizedKey := strings.ToLower(key)
	delete(r.models, normalizedKey)
	r.mu.Unlock()

	// Save the parent aggregate immediately after modification
	if err := r.saveFunc(); err != nil {
		return fmt.Errorf("failed to persist aggregate after delete (context: %s): %w", r.contextKey, err)
	}

	return nil
}

// Count returns the number of models in the repository.
func (r *NestedRepository[T]) Count() int {
	r.mu.RLock()
	defer r.mu.RUnlock()

	return len(r.models)
}

// Save persists changes by calling the saveFunc to save the parent aggregate.
// This is useful when external code needs to explicitly trigger a save.
func (r *NestedRepository[T]) Save() error {
	return r.saveFunc()
}
