package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/serialize"
	"fmt"
	"log"
	"maps"
	"os"
	"slices"
	"sync"
)

// DecodeFunc is a function type that decodes a model from Viper configuration by key
type DecodeFunc[T any] func(key string) (*T, error)

// NewFunc is a function type that creates a new instance of a model
type NewFunc[T any] func() *T

// Repository defines the standard interface for data access operations
type Repository[T any] interface {
	Get(key string) (*T, error)
	GetAll() map[string]*T
	GetAllKeys() []string
	Set(key string, model *T) error
	Delete(key string) error
	Save() error
	Reload() error
	Count() int
}

// BaseRepository provides generic repository functionality with thread-safe operations
// for top-level models that are persisted directly to the configuration file.
//
// # Purpose
//
// BaseRepository implements the Repository pattern for models that are stored at the
// root level of the configuration (e.g., Macros, Programs). It handles:
// - Thread-safe CRUD operations with sync.RWMutex
// - Automatic persistence to disk via Viper
// - Case-insensitive key normalization
// - Lazy loading and reloading from configuration
//
// # When to Use BaseRepository
//
// Use BaseRepository for:
// - Top-level domain models (Macro, Program)
// - Models that are stored directly in config.yaml under their own key
// - Models that don't belong to another aggregate root
// - Models that need independent persistence
//
// Example config structure for BaseRepository models:
//
//	macros:
//	  "my-macro":
//	    name: "my-macro"
//	    hotkey: ["ctrl", "shift", "m"]
//	  "another-macro":
//	    name: "another-macro"
//	    hotkey: ["ctrl", "alt", "a"]
//
// # When NOT to Use BaseRepository
//
// Do NOT use BaseRepository for:
// - Nested models (Items, Points, SearchAreas) - use NestedRepository instead
// - Models that belong to an aggregate root
// - Models that should be saved as part of their parent
//
// See NestedRepository for managing models within an aggregate root context.
//
// # Architecture: BaseRepository vs NestedRepository
//
// BaseRepository (for top-level models):
//   - Persists directly to config via Viper.WriteConfig()
//   - Each model is independent
//   - Example: Macro, Program
//
// NestedRepository (for child models):
//   - Persists by saving the parent aggregate root
//   - Models are part of a larger aggregate
//   - Example: Item (within Program), Point (within Program/Coordinates)
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
// - Write operations (Set, Delete, Save, Reload) use Lock
// - Multiple concurrent reads are allowed
// - Writes are exclusive
//
// # Example Usage
//
// Creating a repository:
//
//	macroRepo := NewBaseRepository[models.Macro](
//	    "macros",                              // Config key in YAML
//	    func(key string) (*models.Macro, error) {
//	        // Decode logic using Viper
//	        return decodeMacro(key)
//	    },
//	    func() *models.Macro {
//	        return &models.Macro{}             // Factory function
//	    },
//	)
//
// Using the repository:
//
//	// Create and store a model
//	macro := &models.Macro{Name: "test"}
//	err := macroRepo.Set("test", macro)        // Saves to disk immediately
//
//	// Retrieve a model
//	retrieved, err := macroRepo.Get("test")    // Case-insensitive
//
//	// List all models
//	allMacros := macroRepo.GetAll()            // Returns a copy
//	keys := macroRepo.GetAllKeys()             // Sorted keys
//
//	// Delete a model
//	err = macroRepo.Delete("test")             // Saves to disk immediately
//
//	// Reload from disk
//	err = macroRepo.Reload()                   // Refreshes from config file
type BaseRepository[T any] struct {
	mu         sync.RWMutex
	models     map[string]*T
	configKey  string
	decodeFunc DecodeFunc[T]
	newFunc    NewFunc[T]
}

// NewBaseRepository creates a new BaseRepository instance.
// The type parameter T should be a type where *T implements models.BaseModel.
// configKey: the key in config.yaml (e.g., "macros", "programs")
// decodeFunc: function to decode a single model from Viper
// newFunc: function to create a new model instance
func NewBaseRepository[T any](configKey string, decodeFunc DecodeFunc[T], newFunc NewFunc[T]) *BaseRepository[T] {
	return &BaseRepository[T]{
		models:     make(map[string]*T),
		configKey:  configKey,
		decodeFunc: decodeFunc,
		newFunc:    newFunc,
	}
}

// Get retrieves a model by key. Returns ErrNotFound if the key doesn't exist.
// Keys must match exactly (case-sensitive).
func (r *BaseRepository[T]) Get(key string) (*T, error) {
	if key == "" {
		return nil, ErrInvalidKey
	}

	r.mu.RLock()
	defer r.mu.RUnlock()

	model, ok := r.models[key]
	if !ok {
		return nil, fmt.Errorf("%w: %s", ErrNotFound, key)
	}

	return model, nil
}

// GetAll returns a copy of all models in the repository.
// This is safe for concurrent access.
func (r *BaseRepository[T]) GetAll() map[string]*T {
	r.mu.RLock()
	defer r.mu.RUnlock()

	// Return a copy to prevent external modification
	result := make(map[string]*T, len(r.models))
	maps.Copy(result, r.models)

	return result
}

// GetAllKeys returns a sorted slice of all keys in the repository
func (r *BaseRepository[T]) GetAllKeys() []string {
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
// Keys are normalized to lowercase. Changes are immediately persisted to disk.
func (r *BaseRepository[T]) Set(key string, model *T) error {
	if key == "" {
		return ErrInvalidKey
	}
	if model == nil {
		return fmt.Errorf("model cannot be nil")
	}

	r.mu.Lock()

	// Synchronize model's internal key with the provided key (exact match)
	if baseModel, ok := any(model).(models.BaseModel); ok {
		baseModel.SetKey(key)
	}

	r.models[key] = model
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist after set: %w", err)
	}

	return nil
}

// Delete removes a model by key. This operation is idempotent.
// Changes are immediately persisted to disk.
func (r *BaseRepository[T]) Delete(key string) error {
	if key == "" {
		return ErrInvalidKey
	}

	r.mu.Lock()
	delete(r.models, key)
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist after delete: %w", err)
	}

	return nil
}

// Save persists all models to disk via YAML
func (r *BaseRepository[T]) Save() error {
	r.mu.RLock()
	defer r.mu.RUnlock()

	yamlConfig := serialize.GetYAMLConfig()
	yamlConfig.Set(r.configKey, r.models)
	if err := yamlConfig.WriteConfig(); err != nil {
		return fmt.Errorf("%w: %v", ErrSaveFailed, err)
	}

	log.Printf("Successfully saved %d %s to disk", len(r.models), r.configKey)
	return nil
}

// Reload refreshes the repository by loading all models from disk.
// Individual decode failures are logged but don't stop the loading process.
// In test mode (SQYRE_TEST_MODE=1), this will re-read the config file from disk.
func (r *BaseRepository[T]) Reload() error {
	yamlConfig := serialize.GetYAMLConfig()

	// In test mode, re-read config file from disk to ensure fresh data
	if os.Getenv("SQYRE_TEST_MODE") == "1" {
		// Re-read from disk
		if err := yamlConfig.ReadConfig(); err != nil {
			return fmt.Errorf("failed to re-read config in test mode: %w", err)
		}
	}

	configMap := yamlConfig.GetStringMap(r.configKey)

	r.mu.Lock()
	defer r.mu.Unlock()

	// Clear existing models
	r.models = make(map[string]*T)

	// Decode each model
	successCount := 0
	for key := range configMap {
		model, err := r.decodeFunc(key)
		if err != nil {
			log.Printf("Warning: failed to decode %s '%s': %v", r.configKey, key, err)
			continue
		}

		r.models[key] = model
		successCount++
	}

	log.Printf("Successfully loaded %d %s from disk", successCount, r.configKey)

	if successCount == 0 && len(configMap) > 0 {
		return fmt.Errorf("%w: no %s could be loaded", ErrLoadFailed, r.configKey)
	}

	return nil
}

// Count returns the number of models in the repository
func (r *BaseRepository[T]) Count() int {
	r.mu.RLock()
	defer r.mu.RUnlock()

	return len(r.models)
}

// GetAllAsStringSlice returns a sorted slice of all keys in the repository.
// This is a convenience method for UI components that need string slices.
// func (r *BaseRepository[T]) GetAllAsStringSlice() []string {
// 	return r.GetAllKeys()
// }

// New creates a new instance of the model using the newFunc.
// This is a convenience method for creating new models.
func (r *BaseRepository[T]) New() *T {
	return r.newFunc()
}
