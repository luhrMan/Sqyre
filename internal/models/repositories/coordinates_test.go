package repositories

import (
	"Squire/internal/models"
	"errors"
	"fmt"
	"sync"
	"testing"
)

// Helper function to create a test program with coordinates
func createTestProgramWithCoordinates(name string, resolutionKey string) *models.Program {
	program := models.NewProgram()
	program.Name = name

	// Initialize coordinates for the resolution
	program.Coordinates[resolutionKey] = &models.Coordinates{
		Points: map[string]*models.Point{
			"stash-screen": {
				Name: "stash-screen",
				X:    1280,
				Y:    100,
			},
			"merchant-button": {
				Name: "merchant-button",
				X:    500,
				Y:    800,
			},
		},
		SearchAreas: map[string]*models.SearchArea{
			"stash-player-inv": {
				Name:    "stash-player-inv",
				LeftX:   100,
				TopY:    200,
				RightX:  500,
				BottomY: 600,
			},
		},
	}

	return program
}

// TestPointRepository_Get tests the Get method with various scenarios
func TestPointRepository_Get(t *testing.T) {
	program := createTestProgramWithCoordinates("test game", "2560x1440")
	repo := NewPointRepository(program, "2560x1440")

	t.Run("Get existing point", func(t *testing.T) {
		point, err := repo.Get("stash-screen")
		if err != nil {
			t.Fatalf("Failed to get existing point: %v", err)
		}

		if point.Name != "stash-screen" {
			t.Errorf("Expected name 'stash-screen', got '%s'", point.Name)
		}

		if point.X != 1280 {
			t.Errorf("Expected X 1280, got %d", point.X)
		}

		if point.Y != 100 {
			t.Errorf("Expected Y 100, got %d", point.Y)
		}
	})

	t.Run("Get with exact key matching", func(t *testing.T) {
		// Test that exact key works
		point, err := repo.Get("stash-screen")
		if err != nil {
			t.Fatalf("Failed to get point with exact key 'stash-screen': %v", err)
		}

		if point.Name != "stash-screen" {
			t.Errorf("Expected point name 'stash-screen', got '%s'", point.Name)
		}

		// Test that different case keys don't work (exact matching only)
		differentCases := []string{"STASH-SCREEN", "Stash-Screen", "StAsH-ScReEn"}
		for _, key := range differentCases {
			_, err := repo.Get(key)
			if !errors.Is(err, ErrNotFound) {
				t.Errorf("Expected ErrNotFound for different case key '%s', got: %v", key, err)
			}
		}
	})

	t.Run("Get non-existent point returns ErrNotFound", func(t *testing.T) {
		_, err := repo.Get("nonexistent")
		if !errors.Is(err, ErrNotFound) {
			t.Errorf("Expected ErrNotFound, got %v", err)
		}

		// Verify error message includes context
		if err != nil && err.Error() == "" {
			t.Error("Error should have a descriptive message")
		}

		// Verify error message includes program name and resolution
		if err != nil {
			errMsg := err.Error()
			if !contains(errMsg, "test game") {
				t.Error("Error message should include program name")
			}
			if !contains(errMsg, "2560x1440") {
				t.Error("Error message should include resolution")
			}
		}
	})

	t.Run("Get with empty name returns ErrInvalidKey", func(t *testing.T) {
		_, err := repo.Get("")
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("Expected ErrInvalidKey for empty name, got %v", err)
		}
	})
}

// TestPointRepository_GetAll tests the GetAll method
func TestPointRepository_GetAll(t *testing.T) {
	program := createTestProgramWithCoordinates("test game", "2560x1440")
	repo := NewPointRepository(program, "2560x1440")

	t.Run("GetAll returns all points", func(t *testing.T) {
		all := repo.GetAll()

		if len(all) != 2 {
			t.Errorf("Expected 2 points, got %d", len(all))
		}

		if _, exists := all["stash-screen"]; !exists {
			t.Error("Expected 'stash-screen' in results")
		}

		if _, exists := all["merchant-button"]; !exists {
			t.Error("Expected 'merchant-button' in results")
		}
	})

	t.Run("GetAll returns a copy", func(t *testing.T) {
		all := repo.GetAll()

		// Modify the returned map
		all["new point"] = &models.Point{Name: "New Point", X: 999, Y: 999}

		// Verify original repository is unchanged
		if repo.Count() != 2 {
			t.Error("GetAll should return a copy, not the original map")
		}

		_, err := repo.Get("new point")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Modifications to GetAll result should not affect repository")
		}
	})

	t.Run("GetAll on empty repository", func(t *testing.T) {
		emptyProgram := models.NewProgram()
		emptyProgram.Name = "empty"
		emptyRepo := NewPointRepository(emptyProgram, "1920x1080")

		all := emptyRepo.GetAll()
		if len(all) != 0 {
			t.Errorf("Expected empty map, got %d points", len(all))
		}
	})
}

// TestPointRepository_GetAllKeys tests the GetAllKeys method
func TestPointRepository_GetAllKeys(t *testing.T) {
	program := createTestProgramWithCoordinates("test game", "2560x1440")
	repo := NewPointRepository(program, "2560x1440")

	t.Run("GetAllKeys returns sorted names", func(t *testing.T) {
		keys := repo.GetAllKeys()

		if len(keys) != 2 {
			t.Errorf("Expected 2 keys, got %d", len(keys))
		}

		// Verify sorting (merchant-button < stash-screen alphabetically)
		expected := []string{"merchant-button", "stash-screen"}
		for i, key := range keys {
			if key != expected[i] {
				t.Errorf("Expected key[%d] = '%s', got '%s'", i, expected[i], key)
			}
		}
	})

	t.Run("GetAllKeys on empty repository", func(t *testing.T) {
		emptyProgram := models.NewProgram()
		emptyProgram.Name = "empty"
		emptyRepo := NewPointRepository(emptyProgram, "1920x1080")

		keys := emptyRepo.GetAllKeys()
		if len(keys) != 0 {
			t.Errorf("Expected empty slice, got %d keys", len(keys))
		}
	})

	t.Run("GetAllKeys with many points maintains sort order", func(t *testing.T) {
		program := models.NewProgram()
		program.Name = "many points"

		program.Coordinates["1920x1080"] = &models.Coordinates{
			Points: map[string]*models.Point{
				"zebra":  {Name: "zebra", X: 1, Y: 1},
				"apple":  {Name: "apple", X: 2, Y: 2},
				"mango":  {Name: "mango", X: 3, Y: 3},
				"banana": {Name: "banana", X: 4, Y: 4},
				"cherry": {Name: "cherry", X: 5, Y: 5},
			},
			SearchAreas: make(map[string]*models.SearchArea),
		}

		repo := NewPointRepository(program, "1920x1080")
		keys := repo.GetAllKeys()

		// Verify alphabetical order
		expected := []string{"apple", "banana", "cherry", "mango", "zebra"}
		for i, key := range keys {
			if key != expected[i] {
				t.Errorf("Expected key[%d] = '%s', got '%s'", i, expected[i], key)
			}
		}
	})
}

// TestPointRepository_Set tests the Set method
func TestPointRepository_Set(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	// Create and save a program first
	program := models.NewProgram()
	program.Name = "Test Game"

	// Save the program to make it available for PointRepository.Save()
	err := ProgramRepo().Set("test game", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	repo := NewPointRepository(program, "2560x1440")

	t.Run("Set creates new point", func(t *testing.T) {
		newPoint := &models.Point{
			Name: "New Point",
			X:    800,
			Y:    600,
		}

		err := repo.Set("new point", newPoint)
		if err != nil {
			t.Fatalf("Failed to set new point: %v", err)
		}

		// Verify point was added
		retrieved, err := repo.Get("new point")
		if err != nil {
			t.Fatalf("Failed to get newly set point: %v", err)
		}

		// After key synchronization, Name should match the provided key exactly
		if retrieved.Name != "new point" {
			t.Errorf("Expected name 'new point', got '%s'", retrieved.Name)
		}

		if retrieved.X != 800 {
			t.Errorf("Expected X 800, got %d", retrieved.X)
		}
	})

	t.Run("Set updates existing point", func(t *testing.T) {
		// Add initial point
		initialPoint := &models.Point{
			Name: "Update Test",
			X:    100,
			Y:    200,
		}
		repo.Set("update test", initialPoint)

		// Update the point
		updatedPoint := &models.Point{
			Name: "Update Test",
			X:    300,
			Y:    400,
		}
		err := repo.Set("update test", updatedPoint)
		if err != nil {
			t.Fatalf("Failed to update point: %v", err)
		}

		// Verify update
		retrieved, err := repo.Get("update test")
		if err != nil {
			t.Fatalf("Failed to get updated point: %v", err)
		}

		if retrieved.X != 300 {
			t.Errorf("Expected X 300, got %d", retrieved.X)
		}

		if retrieved.Y != 400 {
			t.Errorf("Expected Y 400, got %d", retrieved.Y)
		}
	})

	t.Run("Set with empty name returns ErrInvalidKey", func(t *testing.T) {
		point := &models.Point{Name: "Test", X: 1, Y: 1}
		err := repo.Set("", point)
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("Expected ErrInvalidKey for empty name, got %v", err)
		}
	})

	t.Run("Set with nil point returns error", func(t *testing.T) {
		err := repo.Set("nil test", nil)
		if err == nil {
			t.Error("Expected error when setting nil point")
		}
	})

	t.Run("Set preserves exact key case", func(t *testing.T) {
		point := &models.Point{Name: "Case Test", X: 50, Y: 50}
		err := repo.Set("CASE TEST", point)
		if err != nil {
			t.Fatalf("Failed to set point: %v", err)
		}

		// Should be retrievable with exact case only
		retrieved, err := repo.Get("CASE TEST")
		if err != nil {
			t.Error("Point should be retrievable with exact key")
		}

		// Verify the point's name was synchronized to the exact key
		if retrieved.Name != "CASE TEST" {
			t.Errorf("Expected point name to be 'CASE TEST', got '%s'", retrieved.Name)
		}

		// Should NOT be retrievable with different case
		_, err = repo.Get("case test")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Point should NOT be retrievable with different case")
		}
	})
}

// TestPointRepository_Delete tests the Delete method
func TestPointRepository_Delete(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := createTestProgramWithCoordinates("test game", "2560x1440")

	// Save the program
	err := ProgramRepo().Set("test game", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	repo := NewPointRepository(program, "2560x1440")

	t.Run("Delete existing point", func(t *testing.T) {
		initialCount := repo.Count()

		err := repo.Delete("stash-screen")
		if err != nil {
			t.Fatalf("Failed to delete point: %v", err)
		}

		// Verify point was deleted
		_, err = repo.Get("stash-screen")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Point should not exist after deletion")
		}

		if repo.Count() != initialCount-1 {
			t.Errorf("Expected count %d after deletion, got %d", initialCount-1, repo.Count())
		}
	})

	t.Run("Delete is idempotent", func(t *testing.T) {
		// Delete non-existent point should not error
		err := repo.Delete("nonexistent")
		if err != nil {
			t.Errorf("Delete should be idempotent, got error: %v", err)
		}
	})

	t.Run("Delete with empty name returns ErrInvalidKey", func(t *testing.T) {
		err := repo.Delete("")
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("Expected ErrInvalidKey for empty name, got %v", err)
		}
	})

	t.Run("Delete with exact key matching", func(t *testing.T) {
		// Add a point
		point := &models.Point{Name: "Delete Test", X: 10, Y: 10}
		repo.Set("delete test", point)

		// Delete with exact case should work
		err := repo.Delete("delete test")
		if err != nil {
			t.Fatalf("Failed to delete with exact key: %v", err)
		}

		// Verify deletion
		_, err = repo.Get("delete test")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Point should be deleted with exact key")
		}

		// Add another point to test that different case doesn't delete
		point2 := &models.Point{Name: "Another Test", X: 20, Y: 20}
		repo.Set("Another Test", point2)

		// Try to delete with different case - should not work
		err = repo.Delete("another test")
		if err != nil {
			t.Fatalf("Delete with wrong case should not error, but should not delete: %v", err)
		}

		// Verify point still exists
		_, err = repo.Get("Another Test")
		if err != nil {
			t.Error("Point should still exist when deleted with wrong case")
		}
	})
}

// TestPointRepository_Count tests the Count method
func TestPointRepository_Count(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "count test"

	err := ProgramRepo().Set("count test", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewPointRepository(program, "2560x1440")

	t.Run("Count on empty repository", func(t *testing.T) {
		if repo.Count() != 0 {
			t.Errorf("Expected count 0, got %d", repo.Count())
		}
	})

	t.Run("Count after adding points", func(t *testing.T) {
		repo.Set("point1", &models.Point{Name: "Point 1", X: 1, Y: 1})
		if repo.Count() != 1 {
			t.Errorf("Expected count 1, got %d", repo.Count())
		}

		repo.Set("point2", &models.Point{Name: "Point 2", X: 2, Y: 2})
		if repo.Count() != 2 {
			t.Errorf("Expected count 2, got %d", repo.Count())
		}

		repo.Set("point3", &models.Point{Name: "Point 3", X: 3, Y: 3})
		if repo.Count() != 3 {
			t.Errorf("Expected count 3, got %d", repo.Count())
		}
	})

	t.Run("Count after deleting points", func(t *testing.T) {
		initialCount := repo.Count()

		repo.Delete("point1")
		if repo.Count() != initialCount-1 {
			t.Errorf("Expected count %d after delete, got %d", initialCount-1, repo.Count())
		}

		repo.Delete("point2")
		if repo.Count() != initialCount-2 {
			t.Errorf("Expected count %d after second delete, got %d", initialCount-2, repo.Count())
		}
	})

	t.Run("Count after updating point", func(t *testing.T) {
		count := repo.Count()

		// Update existing point
		repo.Set("point3", &models.Point{Name: "Point 3 Updated", X: 30, Y: 30})

		// Count should remain the same
		if repo.Count() != count {
			t.Errorf("Count should not change on update, expected %d, got %d", count, repo.Count())
		}
	})
}

// TestPointRepository_ThreadSafety_ConcurrentReads tests concurrent read operations
func TestPointRepository_ThreadSafety_ConcurrentReads(t *testing.T) {
	program := models.NewProgram()
	program.Name = "concurrent test"

	// Populate with points
	program.Coordinates["2560x1440"] = &models.Coordinates{
		Points:      make(map[string]*models.Point),
		SearchAreas: make(map[string]*models.SearchArea),
	}

	for i := 0; i < 100; i++ {
		key := fmt.Sprintf("point%d", i)
		program.Coordinates["2560x1440"].Points[key] = &models.Point{
			Name: fmt.Sprintf("Point %d", i),
			X:    i * 10,
			Y:    i * 20,
		}
	}

	repo := NewPointRepository(program, "2560x1440")

	var wg sync.WaitGroup
	errors := make(chan error, 100)

	// Concurrent reads
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("point%d", idx)
			point, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("failed to get %s: %w", key, err)
				return
			}
			if point.X != idx*10 {
				errors <- fmt.Errorf("expected X %d, got %d", idx*10, point.X)
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Error(err)
	}
}

// TestPointRepository_ThreadSafety_ConcurrentWrites tests concurrent write operations
func TestPointRepository_ThreadSafety_ConcurrentWrites(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "concurrent writes"

	// Save program first
	err := ProgramRepo().Set("concurrentwrites", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewPointRepository(program, "2560x1440")

	var wg sync.WaitGroup
	errors := make(chan error, 100)

	// Concurrent writes (direct map access to avoid excessive file I/O)
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("point%d", idx)
			point := &models.Point{
				Name: fmt.Sprintf("Point %d", idx),
				X:    idx * 10,
				Y:    idx * 20,
			}

			// Note: We're testing thread safety, not persistence
			// So we directly modify the map to avoid excessive file I/O
			repo.NestedRepository.mu.Lock()
			repo.NestedRepository.models[key] = point
			repo.NestedRepository.mu.Unlock()
		}(i)
	}

	wg.Wait()
	close(errors)

	// Verify all writes succeeded
	if repo.Count() != 100 {
		t.Errorf("Expected 100 points after concurrent writes, got %d", repo.Count())
	}

	for err := range errors {
		t.Error(err)
	}
}

// TestPointRepository_ThreadSafety_MixedOperations tests mixed concurrent reads and writes
func TestPointRepository_ThreadSafety_MixedOperations(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "mixed operations"

	// Populate with initial data
	program.Coordinates["2560x1440"] = &models.Coordinates{
		Points:      make(map[string]*models.Point),
		SearchAreas: make(map[string]*models.SearchArea),
	}

	for i := 0; i < 50; i++ {
		key := fmt.Sprintf("point%d", i)
		program.Coordinates["2560x1440"].Points[key] = &models.Point{
			Name: fmt.Sprintf("Point %d", i),
			X:    i * 10,
			Y:    i * 20,
		}
	}

	// Save program
	err := ProgramRepo().Set("mixedops", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewPointRepository(program, "2560x1440")

	var wg sync.WaitGroup
	errors := make(chan error, 200)

	// Concurrent reads
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("point%d", idx)
			_, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("read failed for %s: %w", key, err)
			}
		}(i)
	}

	// Concurrent writes (direct map access to avoid file I/O)
	for i := 50; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("point%d", idx)
			point := &models.Point{
				Name: fmt.Sprintf("Point %d", idx),
				X:    idx * 10,
				Y:    idx * 20,
			}
			repo.NestedRepository.mu.Lock()
			repo.NestedRepository.models[key] = point
			repo.NestedRepository.mu.Unlock()
		}(i)
	}

	// Concurrent GetAll
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			all := repo.GetAll()
			if len(all) < 50 {
				errors <- fmt.Errorf("GetAll returned too few points: %d", len(all))
			}
		}()
	}

	// Concurrent GetAllKeys
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			keys := repo.GetAllKeys()
			if len(keys) < 50 {
				errors <- fmt.Errorf("GetAllKeys returned too few keys: %d", len(keys))
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
		t.Errorf("Expected 100 points after mixed operations, got %d", repo.Count())
	}
}

// TestPointRepository_MultipleResolutions tests that repositories for different resolutions are independent
func TestPointRepository_MultipleResolutions(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "multi resolution"

	// Save program
	err := ProgramRepo().Set("multi resolution", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	// Create repositories for different resolutions
	repo1080 := NewPointRepository(program, "1920x1080")
	repo1440 := NewPointRepository(program, "2560x1440")

	// Add points to each resolution
	repo1080.Set("point1", &models.Point{Name: "Point 1 1080", X: 100, Y: 100})
	repo1440.Set("point1", &models.Point{Name: "Point 1 1440", X: 200, Y: 200})

	// Verify they're independent
	point1080, err := repo1080.Get("point1")
	if err != nil {
		t.Fatalf("Failed to get point from 1080 repo: %v", err)
	}
	if point1080.X != 100 {
		t.Errorf("Expected X 100 for 1080 repo, got %d", point1080.X)
	}

	point1440, err := repo1440.Get("point1")
	if err != nil {
		t.Fatalf("Failed to get point from 1440 repo: %v", err)
	}
	if point1440.X != 200 {
		t.Errorf("Expected X 200 for 1440 repo, got %d", point1440.X)
	}

	// Verify counts are independent
	if repo1080.Count() != 1 {
		t.Errorf("Expected count 1 for 1080 repo, got %d", repo1080.Count())
	}
	if repo1440.Count() != 1 {
		t.Errorf("Expected count 1 for 1440 repo, got %d", repo1440.Count())
	}
}

// TestSearchAreaRepository_Get tests the Get method with various scenarios
func TestSearchAreaRepository_Get(t *testing.T) {
	program := createTestProgramWithCoordinates("test game", "2560x1440")
	repo := NewSearchAreaRepository(program, "2560x1440")

	t.Run("Get existing search area", func(t *testing.T) {
		area, err := repo.Get("stash-player-inv")
		if err != nil {
			t.Fatalf("Failed to get existing search area: %v", err)
		}

		if area.Name != "stash-player-inv" {
			t.Errorf("Expected name 'stash-player-inv', got '%s'", area.Name)
		}

		if area.LeftX != 100 {
			t.Errorf("Expected LeftX 100, got %d", area.LeftX)
		}

		if area.TopY != 200 {
			t.Errorf("Expected TopY 200, got %d", area.TopY)
		}

		if area.RightX != 500 {
			t.Errorf("Expected RightX 500, got %d", area.RightX)
		}

		if area.BottomY != 600 {
			t.Errorf("Expected BottomY 600, got %d", area.BottomY)
		}
	})

	t.Run("Get with exact key matching", func(t *testing.T) {
		// Test that exact key works
		area, err := repo.Get("stash-player-inv")
		if err != nil {
			t.Fatalf("Failed to get search area with exact key 'stash-player-inv': %v", err)
		}

		if area.Name != "stash-player-inv" {
			t.Errorf("Expected area name 'stash-player-inv', got '%s'", area.Name)
		}

		// Test that different case keys don't work (exact matching only)
		differentCases := []string{"STASH-PLAYER-INV", "Stash-Player-Inv", "StAsH-PlAyEr-InV"}
		for _, key := range differentCases {
			_, err := repo.Get(key)
			if !errors.Is(err, ErrNotFound) {
				t.Errorf("Expected ErrNotFound for different case key '%s', got: %v", key, err)
			}
		}
	})

	t.Run("Get non-existent search area returns ErrNotFound", func(t *testing.T) {
		_, err := repo.Get("nonexistent")
		if !errors.Is(err, ErrNotFound) {
			t.Errorf("Expected ErrNotFound, got %v", err)
		}

		// Verify error message includes context
		if err != nil && err.Error() == "" {
			t.Error("Error should have a descriptive message")
		}

		// Verify error message includes program name and resolution
		if err != nil {
			errMsg := err.Error()
			if !contains(errMsg, "test game") {
				t.Error("Error message should include program name")
			}
			if !contains(errMsg, "2560x1440") {
				t.Error("Error message should include resolution")
			}
		}
	})

	t.Run("Get with empty name returns ErrInvalidKey", func(t *testing.T) {
		_, err := repo.Get("")
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("Expected ErrInvalidKey for empty name, got %v", err)
		}
	})
}

// TestSearchAreaRepository_GetAll tests the GetAll method
func TestSearchAreaRepository_GetAll(t *testing.T) {
	program := createTestProgramWithCoordinates("test game", "2560x1440")
	repo := NewSearchAreaRepository(program, "2560x1440")

	t.Run("GetAll returns all search areas", func(t *testing.T) {
		all := repo.GetAll()

		if len(all) != 1 {
			t.Errorf("Expected 1 search area, got %d", len(all))
		}

		if _, exists := all["stash-player-inv"]; !exists {
			t.Error("Expected 'stash-player-inv' in results")
		}
	})

	t.Run("GetAll returns a copy", func(t *testing.T) {
		all := repo.GetAll()

		// Modify the returned map
		all["new area"] = &models.SearchArea{
			Name:    "New Area",
			LeftX:   999,
			TopY:    999,
			RightX:  1000,
			BottomY: 1000,
		}

		// Verify original repository is unchanged
		if repo.Count() != 1 {
			t.Error("GetAll should return a copy, not the original map")
		}

		_, err := repo.Get("new area")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Modifications to GetAll result should not affect repository")
		}
	})

	t.Run("GetAll on empty repository", func(t *testing.T) {
		emptyProgram := models.NewProgram()
		emptyProgram.Name = "empty"
		emptyRepo := NewSearchAreaRepository(emptyProgram, "1920x1080")

		all := emptyRepo.GetAll()
		if len(all) != 0 {
			t.Errorf("Expected empty map, got %d search areas", len(all))
		}
	})
}

// TestSearchAreaRepository_GetAllKeys tests the GetAllKeys method
func TestSearchAreaRepository_GetAllKeys(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "test game"

	// Save program
	err := ProgramRepo().Set("test game", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	// Add multiple search areas
	repo.Set("zebra-area", &models.SearchArea{Name: "zebra-area", LeftX: 1, TopY: 1, RightX: 10, BottomY: 10})
	repo.Set("apple-area", &models.SearchArea{Name: "apple-area", LeftX: 2, TopY: 2, RightX: 20, BottomY: 20})
	repo.Set("mango-area", &models.SearchArea{Name: "mango-area", LeftX: 3, TopY: 3, RightX: 30, BottomY: 30})

	t.Run("GetAllKeys returns sorted names", func(t *testing.T) {
		keys := repo.GetAllKeys()

		if len(keys) != 3 {
			t.Errorf("Expected 3 keys, got %d", len(keys))
		}

		// Verify sorting
		expected := []string{"apple-area", "mango-area", "zebra-area"}
		for i, key := range keys {
			if key != expected[i] {
				t.Errorf("Expected key[%d] = '%s', got '%s'", i, expected[i], key)
			}
		}
	})

	t.Run("GetAllKeys on empty repository", func(t *testing.T) {
		emptyProgram := models.NewProgram()
		emptyProgram.Name = "empty"
		emptyRepo := NewSearchAreaRepository(emptyProgram, "1920x1080")

		keys := emptyRepo.GetAllKeys()
		if len(keys) != 0 {
			t.Errorf("Expected empty slice, got %d keys", len(keys))
		}
	})
}

// TestSearchAreaRepository_Set tests the Set method
func TestSearchAreaRepository_Set(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	// Create and save a program first
	program := models.NewProgram()
	program.Name = "Test Game"

	// Save the program to make it available for SearchAreaRepository.Save()
	err := ProgramRepo().Set("test game", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	t.Run("Set creates new search area", func(t *testing.T) {
		newArea := &models.SearchArea{
			Name:    "New Area",
			LeftX:   100,
			TopY:    200,
			RightX:  300,
			BottomY: 400,
		}

		err := repo.Set("new area", newArea)
		if err != nil {
			t.Fatalf("Failed to set new search area: %v", err)
		}

		// Verify search area was added
		retrieved, err := repo.Get("new area")
		if err != nil {
			t.Fatalf("Failed to get newly set search area: %v", err)
		}

		// After key synchronization, Name should match the provided key exactly
		if retrieved.Name != "new area" {
			t.Errorf("Expected name 'new area', got '%s'", retrieved.Name)
		}

		if retrieved.LeftX != 100 {
			t.Errorf("Expected LeftX 100, got %d", retrieved.LeftX)
		}

		if retrieved.RightX != 300 {
			t.Errorf("Expected RightX 300, got %d", retrieved.RightX)
		}
	})

	t.Run("Set updates existing search area", func(t *testing.T) {
		// Add initial search area
		initialArea := &models.SearchArea{
			Name:    "Update Test",
			LeftX:   10,
			TopY:    20,
			RightX:  30,
			BottomY: 40,
		}
		repo.Set("update test", initialArea)

		// Update the search area
		updatedArea := &models.SearchArea{
			Name:    "Update Test",
			LeftX:   50,
			TopY:    60,
			RightX:  70,
			BottomY: 80,
		}
		err := repo.Set("update test", updatedArea)
		if err != nil {
			t.Fatalf("Failed to update search area: %v", err)
		}

		// Verify update
		retrieved, err := repo.Get("update test")
		if err != nil {
			t.Fatalf("Failed to get updated search area: %v", err)
		}

		if retrieved.LeftX != 50 {
			t.Errorf("Expected LeftX 50, got %d", retrieved.LeftX)
		}

		if retrieved.BottomY != 80 {
			t.Errorf("Expected BottomY 80, got %d", retrieved.BottomY)
		}
	})

	t.Run("Set with empty name returns ErrInvalidKey", func(t *testing.T) {
		area := &models.SearchArea{Name: "Test", LeftX: 1, TopY: 1, RightX: 2, BottomY: 2}
		err := repo.Set("", area)
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("Expected ErrInvalidKey for empty name, got %v", err)
		}
	})

	t.Run("Set with nil search area returns error", func(t *testing.T) {
		err := repo.Set("nil test", nil)
		if err == nil {
			t.Error("Expected error when setting nil search area")
		}
	})

	t.Run("Set preserves exact key case", func(t *testing.T) {
		area := &models.SearchArea{Name: "Case Test", LeftX: 50, TopY: 50, RightX: 100, BottomY: 100}
		err := repo.Set("CASE TEST", area)
		if err != nil {
			t.Fatalf("Failed to set search area: %v", err)
		}

		// Should be retrievable with exact case only
		retrieved, err := repo.Get("CASE TEST")
		if err != nil {
			t.Error("Search area should be retrievable with exact key")
		}

		// Verify the area's name was synchronized to the exact key
		if retrieved.Name != "CASE TEST" {
			t.Errorf("Expected area name to be 'CASE TEST', got '%s'", retrieved.Name)
		}

		// Should NOT be retrievable with different case
		_, err = repo.Get("case test")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Search area should NOT be retrievable with different case")
		}
	})
}

// TestSearchAreaRepository_Delete tests the Delete method
func TestSearchAreaRepository_Delete(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := createTestProgramWithCoordinates("test game", "2560x1440")

	// Save the program
	err := ProgramRepo().Set("test game", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	t.Run("Delete existing search area", func(t *testing.T) {
		initialCount := repo.Count()

		err := repo.Delete("stash-player-inv")
		if err != nil {
			t.Fatalf("Failed to delete search area: %v", err)
		}

		// Verify search area was deleted
		_, err = repo.Get("stash-player-inv")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Search area should not exist after deletion")
		}

		if repo.Count() != initialCount-1 {
			t.Errorf("Expected count %d after deletion, got %d", initialCount-1, repo.Count())
		}
	})

	t.Run("Delete is idempotent", func(t *testing.T) {
		// Delete non-existent search area should not error
		err := repo.Delete("nonexistent")
		if err != nil {
			t.Errorf("Delete should be idempotent, got error: %v", err)
		}
	})

	t.Run("Delete with empty name returns ErrInvalidKey", func(t *testing.T) {
		err := repo.Delete("")
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("Expected ErrInvalidKey for empty name, got %v", err)
		}
	})

	t.Run("Delete with exact key matching", func(t *testing.T) {
		// Add a search area
		area := &models.SearchArea{Name: "Delete Test", LeftX: 10, TopY: 10, RightX: 20, BottomY: 20}
		repo.Set("delete test", area)

		// Delete with exact case should work
		err := repo.Delete("delete test")
		if err != nil {
			t.Fatalf("Failed to delete with exact key: %v", err)
		}

		// Verify deletion
		_, err = repo.Get("delete test")
		if !errors.Is(err, ErrNotFound) {
			t.Error("Search area should be deleted with exact key")
		}

		// Add another area to test that different case doesn't delete
		area2 := &models.SearchArea{Name: "Another Test", LeftX: 30, TopY: 30, RightX: 40, BottomY: 40}
		repo.Set("Another Test", area2)

		// Try to delete with different case - should not work
		err = repo.Delete("another test")
		if err != nil {
			t.Fatalf("Delete with wrong case should not error, but should not delete: %v", err)
		}

		// Verify area still exists
		_, err = repo.Get("Another Test")
		if err != nil {
			t.Error("Search area should still exist when deleted with wrong case")
		}
	})
}

// TestSearchAreaRepository_Count tests the Count method
func TestSearchAreaRepository_Count(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "count test"

	err := ProgramRepo().Set("count test", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	t.Run("Count on empty repository", func(t *testing.T) {
		if repo.Count() != 0 {
			t.Errorf("Expected count 0, got %d", repo.Count())
		}
	})

	t.Run("Count after adding search areas", func(t *testing.T) {
		repo.Set("area1", &models.SearchArea{Name: "Area 1", LeftX: 1, TopY: 1, RightX: 10, BottomY: 10})
		if repo.Count() != 1 {
			t.Errorf("Expected count 1, got %d", repo.Count())
		}

		repo.Set("area2", &models.SearchArea{Name: "Area 2", LeftX: 2, TopY: 2, RightX: 20, BottomY: 20})
		if repo.Count() != 2 {
			t.Errorf("Expected count 2, got %d", repo.Count())
		}

		repo.Set("area3", &models.SearchArea{Name: "Area 3", LeftX: 3, TopY: 3, RightX: 30, BottomY: 30})
		if repo.Count() != 3 {
			t.Errorf("Expected count 3, got %d", repo.Count())
		}
	})

	t.Run("Count after deleting search areas", func(t *testing.T) {
		initialCount := repo.Count()

		repo.Delete("area1")
		if repo.Count() != initialCount-1 {
			t.Errorf("Expected count %d after delete, got %d", initialCount-1, repo.Count())
		}

		repo.Delete("area2")
		if repo.Count() != initialCount-2 {
			t.Errorf("Expected count %d after second delete, got %d", initialCount-2, repo.Count())
		}
	})

	t.Run("Count after updating search area", func(t *testing.T) {
		count := repo.Count()

		// Update existing search area
		repo.Set("area3", &models.SearchArea{Name: "Area 3 Updated", LeftX: 30, TopY: 30, RightX: 300, BottomY: 300})

		// Count should remain the same
		if repo.Count() != count {
			t.Errorf("Count should not change on update, expected %d, got %d", count, repo.Count())
		}
	})
}

// TestSearchAreaRepository_ThreadSafety_ConcurrentReads tests concurrent read operations
func TestSearchAreaRepository_ThreadSafety_ConcurrentReads(t *testing.T) {
	program := models.NewProgram()
	program.Name = "concurrent test"

	// Populate with search areas
	program.Coordinates["2560x1440"] = &models.Coordinates{
		Points:      make(map[string]*models.Point),
		SearchAreas: make(map[string]*models.SearchArea),
	}

	for i := 0; i < 100; i++ {
		key := fmt.Sprintf("area%d", i)
		program.Coordinates["2560x1440"].SearchAreas[key] = &models.SearchArea{
			Name:    fmt.Sprintf("Area %d", i),
			LeftX:   i * 10,
			TopY:    i * 20,
			RightX:  i*10 + 100,
			BottomY: i*20 + 200,
		}
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	var wg sync.WaitGroup
	errors := make(chan error, 100)

	// Concurrent reads
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("area%d", idx)
			area, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("failed to get %s: %w", key, err)
				return
			}
			if area.LeftX != idx*10 {
				errors <- fmt.Errorf("expected LeftX %d, got %d", idx*10, area.LeftX)
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Error(err)
	}
}

// TestSearchAreaRepository_ThreadSafety_ConcurrentWrites tests concurrent write operations
func TestSearchAreaRepository_ThreadSafety_ConcurrentWrites(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "concurrent writes"

	// Save program first
	err := ProgramRepo().Set("concurrentwrites", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	var wg sync.WaitGroup
	errors := make(chan error, 100)

	// Concurrent writes (direct map access to avoid excessive file I/O)
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("area%d", idx)
			area := &models.SearchArea{
				Name:    fmt.Sprintf("Area %d", idx),
				LeftX:   idx * 10,
				TopY:    idx * 20,
				RightX:  idx*10 + 100,
				BottomY: idx*20 + 200,
			}

			// Note: We're testing thread safety, not persistence
			// So we directly modify the map to avoid excessive file I/O
			repo.NestedRepository.mu.Lock()
			repo.NestedRepository.models[key] = area
			repo.NestedRepository.mu.Unlock()
		}(i)
	}

	wg.Wait()
	close(errors)

	// Verify all writes succeeded
	if repo.Count() != 100 {
		t.Errorf("Expected 100 search areas after concurrent writes, got %d", repo.Count())
	}

	for err := range errors {
		t.Error(err)
	}
}

// TestSearchAreaRepository_ThreadSafety_MixedOperations tests mixed concurrent reads and writes
func TestSearchAreaRepository_ThreadSafety_MixedOperations(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "mixed operations"

	// Populate with initial data
	program.Coordinates["2560x1440"] = &models.Coordinates{
		Points:      make(map[string]*models.Point),
		SearchAreas: make(map[string]*models.SearchArea),
	}

	for i := 0; i < 50; i++ {
		key := fmt.Sprintf("area%d", i)
		program.Coordinates["2560x1440"].SearchAreas[key] = &models.SearchArea{
			Name:    fmt.Sprintf("Area %d", i),
			LeftX:   i * 10,
			TopY:    i * 20,
			RightX:  i*10 + 100,
			BottomY: i*20 + 200,
		}
	}

	// Save program
	err := ProgramRepo().Set("mixedops", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewSearchAreaRepository(program, "2560x1440")

	var wg sync.WaitGroup
	errors := make(chan error, 200)

	// Concurrent reads
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("area%d", idx)
			_, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("read failed for %s: %w", key, err)
			}
		}(i)
	}

	// Concurrent writes (direct map access to avoid file I/O)
	for i := 50; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("area%d", idx)
			area := &models.SearchArea{
				Name:    fmt.Sprintf("Area %d", idx),
				LeftX:   idx * 10,
				TopY:    idx * 20,
				RightX:  idx*10 + 100,
				BottomY: idx*20 + 200,
			}
			repo.NestedRepository.mu.Lock()
			repo.NestedRepository.models[key] = area
			repo.NestedRepository.mu.Unlock()
		}(i)
	}

	// Concurrent GetAll
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			all := repo.GetAll()
			if len(all) < 50 {
				errors <- fmt.Errorf("GetAll returned too few search areas: %d", len(all))
			}
		}()
	}

	// Concurrent GetAllKeys
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			keys := repo.GetAllKeys()
			if len(keys) < 50 {
				errors <- fmt.Errorf("GetAllKeys returned too few keys: %d", len(keys))
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
		t.Errorf("Expected 100 search areas after mixed operations, got %d", repo.Count())
	}
}

// TestSearchAreaRepository_MultipleResolutions tests that repositories for different resolutions are independent
func TestSearchAreaRepository_MultipleResolutions(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "multi resolution"

	// Save program
	err := ProgramRepo().Set("multiresolution", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	// Create repositories for different resolutions
	repo1080 := NewSearchAreaRepository(program, "1920x1080")
	repo1440 := NewSearchAreaRepository(program, "2560x1440")

	// Add search areas to each resolution
	repo1080.Set("area1", &models.SearchArea{Name: "Area 1080", LeftX: 100, TopY: 100, RightX: 200, BottomY: 200})
	repo1440.Set("area1", &models.SearchArea{Name: "Area 1440", LeftX: 200, TopY: 200, RightX: 400, BottomY: 400})

	// Verify they're independent
	area1080, err := repo1080.Get("area1")
	if err != nil {
		t.Fatalf("Failed to get search area from 1080 repo: %v", err)
	}
	if area1080.LeftX != 100 {
		t.Errorf("Expected LeftX 100 for 1080 repo, got %d", area1080.LeftX)
	}

	area1440, err := repo1440.Get("area1")
	if err != nil {
		t.Fatalf("Failed to get search area from 1440 repo: %v", err)
	}
	if area1440.LeftX != 200 {
		t.Errorf("Expected LeftX 200 for 1440 repo, got %d", area1440.LeftX)
	}

	// Verify counts are independent
	if repo1080.Count() != 1 {
		t.Errorf("Expected count 1 for 1080 repo, got %d", repo1080.Count())
	}
	if repo1440.Count() != 1 {
		t.Errorf("Expected count 1 for 1440 repo, got %d", repo1440.Count())
	}
}

// Helper function to check if a string contains a substring
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) && containsAt(s, substr))
}

func containsAt(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

// TestNestedRepository_KeySynchronization_Point verifies that Point key synchronization
// works correctly when using Set() with a different key than the point's Name
func TestNestedRepository_KeySynchronization_Point(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	// Create a Program with a PointRepository for a specific resolution
	program := models.NewProgram()
	program.Name = "Point Key Sync Test"
	resolutionKey := "1920x1080"

	// Initialize coordinates for the resolution
	program.Coordinates[resolutionKey] = &models.Coordinates{
		Points:      make(map[string]*models.Point),
		SearchAreas: make(map[string]*models.SearchArea),
	}

	// Save the program to make it available for PointRepository.Save()
	err := ProgramRepo().Set("pointkeysynctest", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	pointRepo := NewPointRepository(program, resolutionKey)

	// Create a Point with a different name than the key we'll use
	point := &models.Point{
		Name: "Old Point Name",
		X:    100,
		Y:    200,
	}

	// Call Set() with a different key than the point's Name (preserving capitalization)
	err = pointRepo.Set("New Point Name", point)
	if err != nil {
		t.Fatalf("Failed to set point: %v", err)
	}

	// Verify the point's internal Name field is updated to match the provided key (with capitalization)
	if point.GetKey() != "New Point Name" {
		t.Errorf("Expected point.GetKey() to be 'New Point Name', got '%s'", point.GetKey())
	}

	if point.Name != "New Point Name" {
		t.Errorf("Expected point.Name to be 'New Point Name', got '%s'", point.Name)
	}

	// Verify the point can be retrieved with the exact key
	retrieved, err := pointRepo.Get("New Point Name")
	if err != nil {
		t.Fatalf("Failed to get point with new key: %v", err)
	}

	if retrieved.GetKey() != "New Point Name" {
		t.Errorf("Expected retrieved.GetKey() to be 'New Point Name', got '%s'", retrieved.GetKey())
	}

	// Verify other fields are preserved
	if retrieved.X != 100 {
		t.Errorf("Expected X to be 100, got %d", retrieved.X)
	}

	if retrieved.Y != 200 {
		t.Errorf("Expected Y to be 200, got %d", retrieved.Y)
	}

	// Verify the parent Program is saved correctly by getting it from the repository
	// (Set() should have triggered a save through the saveFunc)
	savedProgram, err := ProgramRepo().Get("pointkeysynctest")
	if err != nil {
		t.Fatalf("Failed to get saved program: %v", err)
	}

	// Verify the point exists in the saved program with the correct key
	coords, exists := savedProgram.Coordinates[resolutionKey]
	if !exists {
		t.Fatalf("Coordinates for resolution '%s' should exist in saved program", resolutionKey)
	}

	savedPoint, exists := coords.Points["New Point Name"]
	if !exists {
		t.Fatal("Point should exist in saved program with key 'New Point Name'")
	}

	if savedPoint.Name != "New Point Name" {
		t.Errorf("Expected saved point Name to be 'New Point Name', got '%s'", savedPoint.Name)
	}

	if savedPoint.X != 100 {
		t.Errorf("Expected saved point X to be 100, got %d", savedPoint.X)
	}

	if savedPoint.Y != 200 {
		t.Errorf("Expected saved point Y to be 200, got %d", savedPoint.Y)
	}
}

// TestNestedRepository_KeySynchronization_SearchArea verifies that SearchArea key synchronization
// works correctly when using Set() with a different key than the area's Name
func TestNestedRepository_KeySynchronization_SearchArea(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	// Create a Program with a SearchAreaRepository for a specific resolution
	program := models.NewProgram()
	program.Name = "SearchArea Key Sync Test"
	resolutionKey := "1920x1080"

	// Initialize coordinates for the resolution
	program.Coordinates[resolutionKey] = &models.Coordinates{
		Points:      make(map[string]*models.Point),
		SearchAreas: make(map[string]*models.SearchArea),
	}

	// Save the program to make it available for SearchAreaRepository.Save()
	err := ProgramRepo().Set("areakeysynctest", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	areaRepo := NewSearchAreaRepository(program, resolutionKey)

	// Create a SearchArea with a different name than the key we'll use
	area := &models.SearchArea{
		Name:    "Old Area Name",
		LeftX:   50,
		TopY:    100,
		RightX:  500,
		BottomY: 600,
	}

	// Call Set() with a different key than the area's Name (preserving capitalization)
	err = areaRepo.Set("New Area Name", area)
	if err != nil {
		t.Fatalf("Failed to set search area: %v", err)
	}

	// Verify the area's internal Name field is updated to match the provided key (with capitalization)
	if area.GetKey() != "New Area Name" {
		t.Errorf("Expected area.GetKey() to be 'New Area Name', got '%s'", area.GetKey())
	}

	if area.Name != "New Area Name" {
		t.Errorf("Expected area.Name to be 'New Area Name', got '%s'", area.Name)
	}

	// Verify the area can be retrieved with the exact key
	retrieved, err := areaRepo.Get("New Area Name")
	if err != nil {
		t.Fatalf("Failed to get search area with new key: %v", err)
	}

	if retrieved.GetKey() != "New Area Name" {
		t.Errorf("Expected retrieved.GetKey() to be 'New Area Name', got '%s'", retrieved.GetKey())
	}

	// Verify other fields are preserved
	if retrieved.LeftX != 50 {
		t.Errorf("Expected LeftX to be 50, got %d", retrieved.LeftX)
	}

	if retrieved.TopY != 100 {
		t.Errorf("Expected TopY to be 100, got %d", retrieved.TopY)
	}

	if retrieved.RightX != 500 {
		t.Errorf("Expected RightX to be 500, got %d", retrieved.RightX)
	}

	if retrieved.BottomY != 600 {
		t.Errorf("Expected BottomY to be 600, got %d", retrieved.BottomY)
	}

	// Verify the parent Program is saved correctly by getting it from the repository
	// (Set() should have triggered a save through the saveFunc)
	savedProgram, err := ProgramRepo().Get("areakeysynctest")
	if err != nil {
		t.Fatalf("Failed to get saved program: %v", err)
	}

	// Verify the search area exists in the saved program with the correct key
	coords, exists := savedProgram.Coordinates[resolutionKey]
	if !exists {
		t.Fatalf("Coordinates for resolution '%s' should exist in saved program", resolutionKey)
	}

	savedArea, exists := coords.SearchAreas["New Area Name"]
	if !exists {
		t.Fatal("SearchArea should exist in saved program with key 'New Area Name'")
	}

	if savedArea.Name != "New Area Name" {
		t.Errorf("Expected saved area Name to be 'New Area Name', got '%s'", savedArea.Name)
	}

	if savedArea.LeftX != 50 {
		t.Errorf("Expected saved area LeftX to be 50, got %d", savedArea.LeftX)
	}

	if savedArea.TopY != 100 {
		t.Errorf("Expected saved area TopY to be 100, got %d", savedArea.TopY)
	}

	if savedArea.RightX != 500 {
		t.Errorf("Expected saved area RightX to be 500, got %d", savedArea.RightX)
	}

	if savedArea.BottomY != 600 {
		t.Errorf("Expected saved area BottomY to be 600, got %d", savedArea.BottomY)
	}
}
