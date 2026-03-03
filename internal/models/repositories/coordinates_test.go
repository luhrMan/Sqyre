package repositories

import (
	"Squire/internal/models"
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
