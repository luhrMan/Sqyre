package repositories

import (
	"sync"
	"testing"
)

// resetProgramRepo resets the singleton for testing
func resetProgramRepo() {
	programRepo = nil
	programOnce = sync.Once{}
}

func TestProgramRepo_Singleton(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	// Get repository twice
	repo1 := ProgramRepo()
	repo2 := ProgramRepo()

	// Verify they're the same instance
	if repo1 != repo2 {
		t.Error("ProgramRepo() should return the same singleton instance")
	}
}

func TestProgramRepo_ConcurrentSingletonAccess(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	var wg sync.WaitGroup
	repos := make([]*ProgramRepository, 100)

	// Concurrent access to singleton
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			repos[idx] = ProgramRepo()
		}(i)
	}

	wg.Wait()

	// Verify all references point to the same instance
	first := repos[0]
	for i, repo := range repos {
		if repo != first {
			t.Errorf("Repository at index %d is not the same instance", i)
		}
	}
}

// func TestProgramRepo_LoadFromConfig(t *testing.T) {
// 	setupTestConfig(t)
// 	resetProgramRepo()

// 	repo := ProgramRepo()

// 	// Verify program was loaded
// 	if repo.Count() == 0 {
// 		t.Fatal("Expected programs to be loaded from config")
// 	}

// 	// Get the test program
// 	program, err := repo.Get("test program")
// 	if err != nil {
// 		t.Fatalf("Failed to get test program: %v", err)
// 	}

// 	// Verify program properties
// 	if program.Name != "Test Program" {
// 		t.Errorf("Expected name 'Test Program', got '%s'", program.Name)
// 	}

// 	// Verify Items were loaded
// 	if program.Items == nil {
// 		t.Fatal("Items should not be nil")
// 	}

// 	if len(program.Items) == 0 {
// 		t.Error("Expected items to be loaded")
// 	}

// 	// Verify Coordinates were loaded
// 	if program.Coordinates == nil {
// 		t.Fatal("Coordinates should not be nil")
// 	}

// 	if len(program.Coordinates) == 0 {
// 		t.Error("Expected coordinates to be loaded")
// 	}
// }

// func TestProgramRepo_DecodeNestedStructures(t *testing.T) {
// 	setupTestConfig(t)
// 	resetProgramRepo()

// 	repo := ProgramRepo()

// 	program, err := repo.Get("test program")
// 	if err != nil {
// 		t.Fatalf("Failed to get test program: %v", err)
// 	}

// 	// Verify Items structure
// 	testItem, exists := program.Items["test item"]
// 	if !exists {
// 		t.Fatal("test item should exist in Items")
// 	}

// 	if testItem.Name != "Test Item" {
// 		t.Errorf("Expected item name 'Test Item', got '%s'", testItem.Name)
// 	}

// 	if testItem.GridSize[0] != 1 || testItem.GridSize[1] != 1 {
// 		t.Errorf("Expected GridSize [1, 1], got %v", testItem.GridSize)
// 	}

// 	if len(testItem.Tags) != 2 {
// 		t.Errorf("Expected 2 tags, got %d", len(testItem.Tags))
// 	}

// 	if testItem.StackMax != 10 {
// 		t.Errorf("Expected StackMax 10, got %d", testItem.StackMax)
// 	}

// 	if testItem.Merchant != "test merchant" {
// 		t.Errorf("Expected merchant 'test merchant', got '%s'", testItem.Merchant)
// 	}

// 	// Verify Coordinates structure
// 	coords, exists := program.Coordinates["1920x1080"]
// 	if !exists {
// 		t.Fatal("1920x1080 coordinates should exist")
// 	}

// 	if coords.Points == nil {
// 		t.Fatal("Points should not be nil")
// 	}

// 	testPoint, exists := coords.Points["test point"]
// 	if !exists {
// 		t.Fatal("test point should exist in Points")
// 	}

// 	if testPoint.Name != "Test Point" {
// 		t.Errorf("Expected point name 'Test Point', got '%s'", testPoint.Name)
// 	}

// 	if testPoint.X != 500 || testPoint.Y != 600 {
// 		t.Errorf("Expected point (500, 600), got (%d, %d)", testPoint.X, testPoint.Y)
// 	}

// 	// Verify SearchAreas
// 	if coords.SearchAreas == nil {
// 		t.Fatal("SearchAreas should not be nil")
// 	}

// 	testArea, exists := coords.SearchAreas["test area"]
// 	if !exists {
// 		t.Fatal("testarea should exist in SearchAreas")
// 	}

// 	if testArea.Name != "Test Area" {
// 		t.Errorf("Expected area name 'Test Area', got '%s'", testArea.Name)
// 	}

// 	if testArea.LeftX != 100 || testArea.TopY != 100 {
// 		t.Errorf("Expected area top-left (100, 100), got (%d, %d)", testArea.LeftX, testArea.TopY)
// 	}

// 	if testArea.RightX != 500 || testArea.BottomY != 500 {
// 		t.Errorf("Expected area bottom-right (500, 500), got (%d, %d)", testArea.RightX, testArea.BottomY)
// 	}
// }

// func TestProgramRepo_CRUD(t *testing.T) {
// 	setupTestConfig(t)
// 	resetProgramRepo()

// 	repo := ProgramRepo()

// 	// Create a new program using New()
// 	newProgram := repo.New()
// 	newProgram.Name = "New Program"

// 	// Test Set (skip actual save to avoid file I/O issues)
// 	repo.mu.Lock()
// 	repo.models["new program"] = newProgram
// 	repo.mu.Unlock()

// 	// Test Get
// 	retrieved, err := repo.Get("new program")
// 	if err != nil {
// 		t.Fatalf("Failed to get new program: %v", err)
// 	}

// 	if retrieved.Name != "New Program" {
// 		t.Errorf("Expected name 'New Program', got '%s'", retrieved.Name)
// 	}

// 	// Test case-insensitive access
// 	retrieved, err = repo.Get("New Program")
// 	if err != nil {
// 		t.Errorf("Case-insensitive get failed: %v", err)
// 	}

// 	// Test GetAll
// 	all := repo.GetAll()
// 	if len(all) < 2 {
// 		t.Errorf("Expected at least 2 programs, got %d", len(all))
// 	}

// 	// Test Delete (without save)
// 	repo.mu.Lock()
// 	delete(repo.models, "new program")
// 	repo.mu.Unlock()

// 	_, err = repo.Get("new program")
// 	if !errors.Is(err, ErrNotFound) {
// 		t.Errorf("Expected ErrNotFound after delete, got %v", err)
// 	}
// }

func TestProgramRepo_New(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	repo := ProgramRepo()

	// Test New creates a valid program
	newProgram := repo.New()
	if newProgram == nil {
		t.Fatal("New() returned nil")
	}

	if newProgram.Items == nil {
		t.Error("New program should have Items map initialized")
	}

	if newProgram.Coordinates == nil {
		t.Error("New program should have Coordinates map initialized")
	}

	// Verify default coordinates are created
	if len(newProgram.Coordinates) == 0 {
		t.Error("New program should have default coordinates")
	}
}

func TestDecodeProgram_InvalidKey(t *testing.T) {
	setupTestConfig(t)

	// Try to decode a non-existent program
	// Note: Viper doesn't error on non-existent keys, it returns empty values
	// This test verifies the decode function can handle this gracefully
	program, err := decodeProgram("nonexistent")
	if err != nil {
		t.Errorf("Unexpected error: %v", err)
	}

	// The program will be decoded but with empty/zero values
	if program == nil {
		t.Error("Expected non-nil program even for non-existent key")
	}
}

// func TestProgramRepo_Reload(t *testing.T) {
// 	setupTestConfig(t)
// 	resetProgramRepo()

// 	repo := ProgramRepo()
// 	initialCount := repo.Count()

// 	// Add a program directly to the map (simulating runtime addition)
// 	repo.mu.Lock()
// 	runtimeProgram := repo.New()
// 	runtimeProgram.Name = "Runtime Program"
// 	repo.models["runtime"] = runtimeProgram
// 	repo.mu.Unlock()

// 	if repo.Count() != initialCount+1 {
// 		t.Error("Failed to add runtime program")
// 	}

// 	// Reload should reset to config state
// 	err := repo.Reload()
// 	if err != nil {
// 		t.Fatalf("Reload failed: %v", err)
// 	}

// 	// Runtime program should be gone
// 	_, err = repo.Get("runtime")
// 	if !errors.Is(err, ErrNotFound) {
// 		t.Error("Runtime program should not exist after reload")
// 	}

// 	// Original program should still exist
// 	_, err = repo.Get("test program")
// 	if err != nil {
// 		t.Errorf("Original program should exist after reload: %v", err)
// 	}
// }

// func TestProgramRepo_ItemsAggregateAccess(t *testing.T) {
// 	setupTestConfig(t)
// 	resetProgramRepo()

// 	repo := ProgramRepo()

// 	// Get program
// 	program, err := repo.Get("test program")
// 	if err != nil {
// 		t.Fatalf("Failed to get test program: %v", err)
// 	}

// 	// Access items through the program (aggregate root pattern)
// 	item, exists := program.Items["test item"]
// 	if !exists {
// 		t.Fatal("Should be able to access items through program")
// 	}

// 	if item.Name != "Test Item" {
// 		t.Errorf("Expected 'Test Item', got '%s'", item.Name)
// 	}

// 	// Modify items through the program
// 	program.Items["newitem"] = &models.Item{
// 		Name:     "New Item",
// 		GridSize: [2]int{2, 2},
// 		Tags:     []string{"new"},
// 		StackMax: 5,
// 		Merchant: "new merchant",
// 	}

// 	if len(program.Items) != 2 {
// 		t.Errorf("Expected 2 items after addition, got %d", len(program.Items))
// 	}
// }

// func TestProgramRepo_CoordinatesAggregateAccess(t *testing.T) {
// 	setupTestConfig(t)
// 	resetProgramRepo()

// 	repo := ProgramRepo()

// 	// Get program
// 	program, err := repo.Get("test program")
// 	if err != nil {
// 		t.Fatalf("Failed to get test program: %v", err)
// 	}

// 	// Access coordinates through the program (aggregate root pattern)
// 	coords, exists := program.Coordinates["1920x1080"]
// 	if !exists {
// 		t.Fatal("Should be able to access coordinates through program")
// 	}

// 	if coords.Points == nil {
// 		t.Fatal("Points should be initialized")
// 	}

// 	// Verify we can access nested structures
// 	point, exists := coords.Points["test point"]
// 	if !exists {
// 		t.Fatal("Should be able to access points through coordinates")
// 	}

// 	if point.Name != "Test Point" {
// 		t.Errorf("Expected 'Test Point', got '%s'", point.Name)
// 	}
// }
