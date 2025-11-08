package repositories

import (
	"Squire/internal/models/serialize"
	"fmt"
	"log"
	"maps"
	"os"
	"slices"
	"strings"
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
type BaseRepository[T any] struct {
	mu         sync.RWMutex
	models     map[string]*T
	configKey  string
	decodeFunc DecodeFunc[T]
	newFunc    NewFunc[T]
}

// NewBaseRepository creates a new BaseRepository instance
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
// Keys are normalized to lowercase for case-insensitive access.
func (r *BaseRepository[T]) Get(key string) (*T, error) {
	if key == "" {
		return nil, ErrInvalidKey
	}

	r.mu.RLock()
	defer r.mu.RUnlock()

	normalizedKey := strings.ToLower(key)
	model, ok := r.models[normalizedKey]
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
	normalizedKey := strings.ToLower(key)
	r.models[normalizedKey] = model
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
	normalizedKey := strings.ToLower(key)
	delete(r.models, normalizedKey)
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist after delete: %w", err)
	}

	return nil
}

// Save persists all models to disk via Viper
func (r *BaseRepository[T]) Save() error {
	r.mu.RLock()
	defer r.mu.RUnlock()

	serialize.GetViper().Set(r.configKey, r.models)
	if err := serialize.GetViper().WriteConfig(); err != nil {
		return fmt.Errorf("%w: %v", ErrSaveFailed, err)
	}

	log.Printf("Successfully saved %d %s to disk", len(r.models), r.configKey)
	return nil
}

// Reload refreshes the repository by loading all models from disk.
// Individual decode failures are logged but don't stop the loading process.
// In test mode (SQYRE_TEST_MODE=1), this will re-read the config file from disk.
func (r *BaseRepository[T]) Reload() error {
	viper := serialize.GetViper()

	// In test mode, re-read config file from disk to ensure fresh data
	// This is necessary because after WriteConfig(), Viper's in-memory state
	// contains struct pointers which GetStringMap() cannot handle properly
	if os.Getenv("SQYRE_TEST_MODE") == "1" {
		// Clear the in-memory config to force a fresh read
		configFile := viper.ConfigFileUsed()
		viper.Set(r.configKey, nil)

		// Re-read from disk
		viper.SetConfigFile(configFile)
		if err := viper.ReadInConfig(); err != nil {
			return fmt.Errorf("failed to re-read config in test mode: %w", err)
		}
	}

	configMap := viper.GetStringMap(r.configKey)

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

		normalizedKey := strings.ToLower(key)
		r.models[normalizedKey] = model
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
