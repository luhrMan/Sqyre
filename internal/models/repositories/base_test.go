package repositories

import (
	"errors"
	"fmt"
	"sync"
	"testing"
)

// testModel is a simple model for testing BaseRepository
type testModel struct {
	Name  string
	Value int
}

// newTestModel creates a new testModel instance
func newTestModel() *testModel {
	return &testModel{}
}

// testDecodeFunc is a mock decode function for testing
func testDecodeFunc(key string) (*testModel, error) {
	if key == "error" {
		return nil, fmt.Errorf("decode error")
	}
	return &testModel{Name: key, Value: 42}, nil
}

func TestNewBaseRepository(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	if repo == nil {
		t.Fatal("NewBaseRepository returned nil")
	}

	if repo.configKey != "test" {
		t.Errorf("Expected configKey 'test', got '%s'", repo.configKey)
	}

	if repo.models == nil {
		t.Error("models map should be initialized")
	}

	if repo.Count() != 0 {
		t.Errorf("Expected empty repository, got count %d", repo.Count())
	}
}

func TestBaseRepository_Get(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Test getting non-existent key
	_, err := repo.Get("nonexistent")
	if !errors.Is(err, ErrNotFound) {
		t.Errorf("Expected ErrNotFound, got %v", err)
	}

	// Add a model
	model := &testModel{Name: "test", Value: 100}
	repo.models["test"] = model

	// Test getting existing key
	retrieved, err := repo.Get("test")
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if retrieved.Name != "test" || retrieved.Value != 100 {
		t.Errorf("Retrieved model doesn't match: %+v", retrieved)
	}

	// Test empty key
	_, err = repo.Get("")
	if !errors.Is(err, ErrInvalidKey) {
		t.Errorf("Expected ErrInvalidKey for empty key, got %v", err)
	}
}

func TestBaseRepository_KeyNormalization(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Add model with lowercase key
	model := &testModel{Name: "test", Value: 100}
	repo.models["test"] = model

	// Test case-insensitive retrieval
	testCases := []string{"test", "Test", "TEST", "TeSt"}
	for _, key := range testCases {
		retrieved, err := repo.Get(key)
		if err != nil {
			t.Errorf("Failed to get with key '%s': %v", key, err)
			continue
		}

		if retrieved.Name != "test" {
			t.Errorf("Key '%s' retrieved wrong model: %+v", key, retrieved)
		}
	}
}

func TestBaseRepository_GetAll(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Test empty repository
	all := repo.GetAll()
	if len(all) != 0 {
		t.Errorf("Expected empty map, got %d items", len(all))
	}

	// Add models
	repo.models["model1"] = &testModel{Name: "model1", Value: 1}
	repo.models["model2"] = &testModel{Name: "model2", Value: 2}

	// Test GetAll returns all models
	all = repo.GetAll()
	if len(all) != 2 {
		t.Errorf("Expected 2 models, got %d", len(all))
	}

	// Verify it's a copy (modifying returned map shouldn't affect repository)
	all["model3"] = &testModel{Name: "model3", Value: 3}
	if repo.Count() != 2 {
		t.Error("GetAll should return a copy, not the original map")
	}
}

func TestBaseRepository_GetAllKeys(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Test empty repository
	keys := repo.GetAllKeys()
	if len(keys) != 0 {
		t.Errorf("Expected empty slice, got %d keys", len(keys))
	}

	// Add models
	repo.models["zebra"] = &testModel{Name: "zebra", Value: 1}
	repo.models["alpha"] = &testModel{Name: "alpha", Value: 2}
	repo.models["beta"] = &testModel{Name: "beta", Value: 3}

	// Test GetAllKeys returns sorted keys
	keys = repo.GetAllKeys()
	if len(keys) != 3 {
		t.Errorf("Expected 3 keys, got %d", len(keys))
	}

	// Verify sorting
	expected := []string{"alpha", "beta", "zebra"}
	for i, key := range keys {
		if key != expected[i] {
			t.Errorf("Expected key[%d] = '%s', got '%s'", i, expected[i], key)
		}
	}
}

func TestBaseRepository_Count(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	if repo.Count() != 0 {
		t.Errorf("Expected count 0, got %d", repo.Count())
	}

	repo.models["model1"] = &testModel{Name: "model1", Value: 1}
	if repo.Count() != 1 {
		t.Errorf("Expected count 1, got %d", repo.Count())
	}

	repo.models["model2"] = &testModel{Name: "model2", Value: 2}
	if repo.Count() != 2 {
		t.Errorf("Expected count 2, got %d", repo.Count())
	}

	delete(repo.models, "model1")
	if repo.Count() != 1 {
		t.Errorf("Expected count 1 after delete, got %d", repo.Count())
	}
}

func TestBaseRepository_New(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	model := repo.New()
	if model == nil {
		t.Fatal("New() returned nil")
	}

	// Verify it's a new instance
	if model.Name != "" || model.Value != 0 {
		t.Errorf("Expected zero-value model, got %+v", model)
	}
}

func TestBaseRepository_ThreadSafety_ConcurrentReads(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Populate repository
	for i := 0; i < 100; i++ {
		key := fmt.Sprintf("model%d", i)
		repo.models[key] = &testModel{Name: key, Value: i}
	}

	// Concurrent reads
	var wg sync.WaitGroup
	errors := make(chan error, 100)

	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("model%d", idx)
			model, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("failed to get %s: %w", key, err)
				return
			}
			if model.Value != idx {
				errors <- fmt.Errorf("expected value %d, got %d", idx, model.Value)
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Error(err)
	}
}

func TestBaseRepository_ThreadSafety_ConcurrentWrites(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Concurrent writes (without Save to avoid file I/O)
	var wg sync.WaitGroup
	errors := make(chan error, 100)

	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("model%d", idx)
			repo.mu.Lock()
			repo.models[key] = &testModel{Name: key, Value: idx}
			repo.mu.Unlock()
		}(i)
	}

	wg.Wait()
	close(errors)

	// Verify all writes succeeded
	if repo.Count() != 100 {
		t.Errorf("Expected 100 models after concurrent writes, got %d", repo.Count())
	}

	for err := range errors {
		t.Error(err)
	}
}

func TestBaseRepository_ThreadSafety_MixedOperations(t *testing.T) {
	repo := NewBaseRepository[testModel]("test", testDecodeFunc, newTestModel)

	// Populate with initial data
	for i := 0; i < 50; i++ {
		key := fmt.Sprintf("model%d", i)
		repo.models[key] = &testModel{Name: key, Value: i}
	}

	var wg sync.WaitGroup
	errors := make(chan error, 200)

	// Concurrent reads
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("model%d", idx)
			_, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("read failed for %s: %w", key, err)
			}
		}(i)
	}

	// Concurrent writes
	for i := 50; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("model%d", idx)
			repo.mu.Lock()
			repo.models[key] = &testModel{Name: key, Value: idx}
			repo.mu.Unlock()
		}(i)
	}

	// Concurrent GetAll
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			all := repo.GetAll()
			if len(all) < 50 {
				errors <- fmt.Errorf("GetAll returned too few items: %d", len(all))
			}
		}()
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Error(err)
	}

	// Final count should be 100
	if repo.Count() != 100 {
		t.Errorf("Expected 100 models after mixed operations, got %d", repo.Count())
	}
}
