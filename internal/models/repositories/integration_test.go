package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/serialize"
	"os"
	"path/filepath"
	"sync"
	"testing"
	"time"
)

// setupIntegrationTest creates a temporary config file for integration testing
func setupIntegrationTest(t *testing.T) (string, func()) {
	t.Helper()

	// Enable test mode for Reload() to re-read config from disk
	os.Setenv("SQYRE_TEST_MODE", "1")

	// Create temporary directory
	// tempDir, err := os.MkdirTemp("", "sqyre-integration-test-*")
	// if err != nil {
	// 	t.Fatalf("Failed to create temp dir: %v", err)
	// }

	// Create config file path

	setupTestConfig(t)
	// Configure Viper to use temp config
	// testdataPath, _ := filepath.Abs("testdata")
	// viper.SetConfigFile(testdataPath)
	// viper.AddConfigPath(testdataPath)
	// viper.SetConfigName("config")
	// viper.SetConfigType("yaml")
	// viper.ReadInConfig()
	// viper.SetConfigName("writeable-config")
	// viper.WriteConfig()

	// Cleanup function
	cleanup := func() {
		// Disable test mode
		os.Unsetenv("SQYRE_TEST_MODE")

		viper := serialize.GetViper()
		// Reset viper to testdata config for other tests
		testdataPath, _ := filepath.Abs("testdata")
		viper.AddConfigPath(testdataPath)
		viper.SetConfigName("config")
		viper.SetConfigType("yaml")
		viper.ReadInConfig()
		viper.SetConfigName("writeable-config")
		viper.WriteConfig()

		// os.RemoveAll(tempDir)
	}
	configPath, _ := filepath.Abs("testdata/writeable-config.yaml")
	return configPath, cleanup
}

func TestIntegration_FullSaveAndReloadCycle(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetMacroRepo()
	resetProgramRepo()

	macroRepo := MacroRepo()
	programRepo := ProgramRepo()

	// Verify initial load
	if macroRepo.Count() != 2 {
		t.Fatalf("Expected 2 macro initially, got %d", macroRepo.Count())
	}
	if programRepo.Count() != 2 {
		t.Fatalf("Expected 2 program initially, got %d", programRepo.Count())
	}

	// Add new macro
	newMacro := models.NewMacro("New Integration Macro", 75, []string{"ctrl", "shift", "n"})
	waitAction := actions.NewWait(250)
	newMacro.Root.SetSubActions([]actions.ActionInterface{waitAction})

	macroRepo.mu.Lock()
	macroRepo.models["newmacro"] = newMacro
	macroRepo.mu.Unlock()

	// Add new program
	newProgram := programRepo.New()
	newProgram.Name = "New Integration Program"
	newProgram.Items["newitem"] = &models.Item{
		Name:     "New Item",
		GridSize: [2]int{1, 2},
		Tags:     []string{"new"},
		StackMax: 15,
		Merchant: "new merchant",
	}

	programRepo.mu.Lock()
	programRepo.models["newprogram"] = newProgram
	programRepo.mu.Unlock()

	// Save both
	if err := macroRepo.Save(); err != nil {
		t.Fatalf("Failed to save macros: %v", err)
	}
	if err := programRepo.Save(); err != nil {
		t.Fatalf("Failed to save programs: %v", err)
	}

	// Reload both (will re-read config from disk in test mode)
	if err := macroRepo.Reload(); err != nil {
		t.Fatalf("Failed to reload macros: %v", err)
	}
	if err := programRepo.Reload(); err != nil {
		t.Fatalf("Failed to reload programs: %v", err)
	}

	// Verify counts
	if macroRepo.Count() != 3 {
		t.Errorf("Expected 3 macros after reload, got %d", macroRepo.Count())
	}
	if programRepo.Count() != 3 {
		t.Errorf("Expected 3 programs after reload, got %d", programRepo.Count())
	}

	// Verify new macro persisted
	reloadedMacro, err := macroRepo.Get("newmacro")
	if err != nil {
		t.Fatalf("Failed to get new macro: %v", err)
	}
	if reloadedMacro.Name != "New Integration Macro" {
		t.Errorf("Expected 'New Integration Macro', got '%s'", reloadedMacro.Name)
	}

	// Verify new program persisted
	reloadedProgram, err := programRepo.Get("newprogram")
	if err != nil {
		t.Fatalf("Failed to get new program: %v", err)
	}
	if reloadedProgram.Name != "New Integration Program" {
		t.Errorf("Expected 'New Integration Program', got '%s'", reloadedProgram.Name)
	}
}

func TestIntegration_ConcurrentRepositoryAccess(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetMacroRepo()
	resetProgramRepo()

	macroRepo := MacroRepo()
	programRepo := ProgramRepo()

	var wg sync.WaitGroup
	errors := make(chan error, 200)

	// Concurrent reads
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_, err := macroRepo.Get("integration test macro")
			if err != nil {
				errors <- err
			}
		}()

		wg.Add(1)
		go func() {
			defer wg.Done()
			_, err := programRepo.Get("integration test program")
			if err != nil {
				errors <- err
			}
		}()
	}

	// Concurrent writes
	for i := 0; i < 25; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			macro := models.NewMacro("Concurrent Macro", 100, []string{"ctrl"})
			macroRepo.mu.Lock()
			macroRepo.models[string(rune('a'+idx))] = macro
			macroRepo.mu.Unlock()
		}(i)

		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			program := programRepo.New()
			program.Name = "Concurrent Program"
			programRepo.mu.Lock()
			programRepo.models[string(rune('a'+idx))] = program
			programRepo.mu.Unlock()
		}(i)
	}

	// Concurrent GetAll operations
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_ = macroRepo.GetAll()
			_ = programRepo.GetAll()
		}()
	}

	wg.Wait()
	close(errors)

	// Check for errors
	for err := range errors {
		t.Error(err)
	}

	// Verify repositories are in consistent state
	if macroRepo.Count() < 1 {
		t.Error("Macro repository should have at least 1 macro")
	}
	if programRepo.Count() < 1 {
		t.Error("Program repository should have at least 1 program")
	}
}

func TestIntegration_MultipleRepositoryInstances(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetMacroRepo()
	resetProgramRepo()

	// Get multiple references to repositories
	macroRepo1 := MacroRepo()
	macroRepo2 := MacroRepo()
	programRepo1 := ProgramRepo()
	programRepo2 := ProgramRepo()

	// Verify they're the same instances (singleton pattern)
	if macroRepo1 != macroRepo2 {
		t.Error("MacroRepo should return the same singleton instance")
	}
	if programRepo1 != programRepo2 {
		t.Error("ProgramRepo should return the same singleton instance")
	}

	// Modify through one reference
	newMacro := models.NewMacro("Shared Macro", 50, []string{"ctrl", "s"})
	macroRepo1.mu.Lock()
	macroRepo1.models["sharedmacro"] = newMacro
	macroRepo1.mu.Unlock()

	// Verify visible through other reference
	retrieved, err := macroRepo2.Get("sharedmacro")
	if err != nil {
		t.Fatalf("Failed to get shared macro: %v", err)
	}
	if retrieved.Name != "Shared Macro" {
		t.Errorf("Expected 'Shared Macro', got '%s'", retrieved.Name)
	}
}

func TestIntegration_SaveReloadCycle_WithModifications(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetMacroRepo()
	resetProgramRepo()

	macroRepo := MacroRepo()
	programRepo := ProgramRepo()

	// Get initial counts
	initialMacroCount := macroRepo.Count()
	initialProgramCount := programRepo.Count()

	// Modify macro
	macro, err := macroRepo.Get("integration test macro")
	if err != nil {
		t.Fatalf("Failed to get macro: %v", err)
	}
	macro.GlobalDelay = 999

	macroRepo.mu.Lock()
	macroRepo.models["integration test macro"] = macro
	macroRepo.mu.Unlock()

	// Modify program
	program, err := programRepo.Get("integration test program")
	if err != nil {
		t.Fatalf("Failed to get program: %v", err)
	}
	program.Items["modifieditem"] = &models.Item{
		Name:     "Modified Item",
		GridSize: [2]int{3, 3},
		Tags:     []string{"modified"},
		StackMax: 99,
		Merchant: "modified merchant",
	}

	programRepo.mu.Lock()
	programRepo.models["integration test program"] = program
	programRepo.mu.Unlock()

	// Save both repositories
	if err := macroRepo.Save(); err != nil {
		t.Fatalf("Failed to save macros: %v", err)
	}
	if err := programRepo.Save(); err != nil {
		t.Fatalf("Failed to save programs: %v", err)
	}

	// Reload both repositories (will re-read config from disk in test mode)
	if err := macroRepo.Reload(); err != nil {
		t.Fatalf("Failed to reload macros: %v", err)
	}
	if err := programRepo.Reload(); err != nil {
		t.Fatalf("Failed to reload programs: %v", err)
	}

	// Verify counts unchanged
	if macroRepo.Count() != initialMacroCount {
		t.Errorf("Macro count changed after reload: expected %d, got %d", initialMacroCount, macroRepo.Count())
	}
	if programRepo.Count() != initialProgramCount {
		t.Errorf("Program count changed after reload: expected %d, got %d", initialProgramCount, programRepo.Count())
	}

	// Verify macro modifications persisted
	reloadedMacro, err := macroRepo.Get("integration test macro")
	if err != nil {
		t.Fatalf("Failed to get macro after reload: %v", err)
	}
	if reloadedMacro.GlobalDelay != 999 {
		t.Errorf("Macro modification not persisted: expected GlobalDelay 999, got %d", reloadedMacro.GlobalDelay)
	}

	// Verify program modifications persisted
	reloadedProgram, err := programRepo.Get("integration test program")
	if err != nil {
		t.Fatalf("Failed to get program after reload: %v", err)
	}
	modifiedItem, exists := reloadedProgram.Items["modifieditem"]
	if !exists {
		t.Fatal("Modified item should exist after reload")
	}
	if modifiedItem.Name != "Modified Item" {
		t.Errorf("Expected 'Modified Item', got '%s'", modifiedItem.Name)
	}
	if modifiedItem.StackMax != 99 {
		t.Errorf("Expected StackMax 99, got %d", modifiedItem.StackMax)
	}
}

func TestIntegration_SequentialSaveOperations(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetMacroRepo()
	resetProgramRepo()

	macroRepo := MacroRepo()
	programRepo := ProgramRepo()

	// Add some test data
	for i := 0; i < 5; i++ {
		macro := models.NewMacro("Save Test Macro", 100, []string{"ctrl"})
		macroRepo.mu.Lock()
		macroRepo.models[string(rune('m'+i))] = macro
		macroRepo.mu.Unlock()

		program := programRepo.New()
		program.Name = "Save Test Program"
		programRepo.mu.Lock()
		programRepo.models[string(rune('p'+i))] = program
		programRepo.mu.Unlock()
	}

	// Sequential saves (Viper is not thread-safe for concurrent writes)
	if err := macroRepo.Save(); err != nil {
		t.Fatalf("Failed to save macros: %v", err)
	}
	if err := programRepo.Save(); err != nil {
		t.Fatalf("Failed to save programs: %v", err)
	}

	// Reload and verify data integrity (will re-read config from disk in test mode)
	if err := macroRepo.Reload(); err != nil {
		t.Fatalf("Failed to reload macros: %v", err)
	}
	if err := programRepo.Reload(); err != nil {
		t.Fatalf("Failed to reload programs: %v", err)
	}

	// Verify data is still consistent
	if macroRepo.Count() < 5 {
		t.Errorf("Expected at least 5 macros, got %d", macroRepo.Count())
	}
	if programRepo.Count() < 5 {
		t.Errorf("Expected at least 5 programs, got %d", programRepo.Count())
	}
}

func TestIntegration_StressTest_RapidOperations(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping stress test in short mode")
	}

	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetMacroRepo()

	repo := MacroRepo()

	var wg sync.WaitGroup
	errors := make(chan error, 1000)

	// Rapid concurrent operations
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()

			// Create
			macro := models.NewMacro("Stress Macro", 50, []string{"ctrl"})
			repo.mu.Lock()
			repo.models[string(rune('s'+idx%26))] = macro
			repo.mu.Unlock()

			// Small delay
			time.Sleep(time.Millisecond)

			// Read
			_, err := repo.Get(string(rune('s' + idx%26)))
			if err != nil {
				errors <- err
			}

			// GetAll
			_ = repo.GetAll()
		}(i)
	}

	wg.Wait()
	close(errors)

	// Check for errors
	errorCount := 0
	for err := range errors {
		t.Error(err)
		errorCount++
		if errorCount > 10 {
			t.Fatal("Too many errors in stress test")
		}
	}

	// Verify repository is still functional
	if repo.Count() < 1 {
		t.Error("Repository should have data after stress test")
	}

	// Verify we can still perform operations
	_, err := repo.Get("integration test macro")
	if err != nil {
		t.Errorf("Repository should still be functional after stress test: %v", err)
	}
}

func TestIntegration_NestedStructurePersistence(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	// Reset singletons
	resetProgramRepo()

	repo := ProgramRepo()

	// Create program with complex nested structures
	program := repo.New()
	program.Name = "Complex Program"

	// Add multiple items
	program.Items["item1"] = &models.Item{
		Name:     "Item 1",
		GridSize: [2]int{1, 1},
		Tags:     []string{"tag1", "tag2"},
		StackMax: 10,
		Merchant: "merchant1",
	}
	program.Items["item2"] = &models.Item{
		Name:     "Item 2",
		GridSize: [2]int{2, 2},
		Tags:     []string{"tag3"},
		StackMax: 5,
		Merchant: "merchant2",
	}

	// Add coordinates with multiple points and areas
	program.Coordinates["1920x1080"] = &models.Coordinates{
		Points: map[string]*models.Point{
			"point1": {Name: "Point 1", X: 100, Y: 200},
			"point2": {Name: "Point 2", X: 300, Y: 400},
		},
		SearchAreas: map[string]*models.SearchArea{
			"area1": {Name: "Area 1", LeftX: 0, TopY: 0, RightX: 500, BottomY: 500},
			"area2": {Name: "Area 2", LeftX: 500, TopY: 500, RightX: 1000, BottomY: 1000},
		},
	}

	repo.mu.Lock()
	repo.models["complexprogram"] = program
	repo.mu.Unlock()

	// Save
	if err := repo.Save(); err != nil {
		t.Fatalf("Failed to save: %v", err)
	}

	// Reload (will re-read config from disk in test mode)
	if err := repo.Reload(); err != nil {
		t.Fatalf("Failed to reload: %v", err)
	}

	// Verify all nested structures persisted
	reloaded, err := repo.Get("complexprogram")
	if err != nil {
		t.Fatalf("Failed to get reloaded program: %v", err)
	}

	if len(reloaded.Items) != 2 {
		t.Errorf("Expected 2 items, got %d", len(reloaded.Items))
	}
	if len(reloaded.Coordinates["1920x1080"].Points) != 2 {
		t.Errorf("Expected 2 points, got %d", len(reloaded.Coordinates["1920x1080"].Points))
	}
	if len(reloaded.Coordinates["1920x1080"].SearchAreas) != 2 {
		t.Errorf("Expected 2 search areas, got %d", len(reloaded.Coordinates["1920x1080"].SearchAreas))
	}
}

// TestIntegration_CoordinateRepositories_LazyInitialization tests lazy initialization of coordinate repositories
func TestIntegration_CoordinateRepositories_LazyInitialization(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("PointRepo lazy initialization", func(t *testing.T) {
		// Create a new program
		program := models.NewProgram()
		program.Name = "Lazy Init Test"

		// Save the program
		err := ProgramRepo().Set("lazyinittest", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program back
		retrieved, err := ProgramRepo().Get("lazyinittest")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Access PointRepo for the first time (should initialize lazily)
		pointRepo := retrieved.PointRepo("1920x1080")
		if pointRepo == nil {
			t.Fatal("PointRepo should not be nil after lazy initialization")
		}

		// Verify it's empty initially
		if pointRepo.Count() != 0 {
			t.Errorf("Expected empty repository, got count %d", pointRepo.Count())
		}

		// Add a point
		err = pointRepo.Set("test-point", &models.Point{Name: "Test Point", X: 100, Y: 200})
		if err != nil {
			t.Fatalf("Failed to set point: %v", err)
		}

		// Access the same repository again (should return the same instance)
		pointRepo2 := retrieved.PointRepo("1920x1080")
		if pointRepo2.Count() != 1 {
			t.Errorf("Expected count 1 from cached repository, got %d", pointRepo2.Count())
		}

		// Verify the point exists
		point, err := pointRepo2.Get("test-point")
		if err != nil {
			t.Fatalf("Failed to get point from cached repository: %v", err)
		}
		if point.X != 100 {
			t.Errorf("Expected X 100, got %d", point.X)
		}
	})

	t.Run("SearchAreaRepo lazy initialization", func(t *testing.T) {
		// Create a new program
		program := models.NewProgram()
		program.Name = "Lazy Init Search Area Test"

		// Save the program
		err := ProgramRepo().Set("lazyinitsearcharea", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program back
		retrieved, err := ProgramRepo().Get("lazyinitsearcharea")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Access SearchAreaRepo for the first time (should initialize lazily)
		searchAreaRepo := retrieved.SearchAreaRepo("2560x1440")
		if searchAreaRepo == nil {
			t.Fatal("SearchAreaRepo should not be nil after lazy initialization")
		}

		// Verify it's empty initially
		if searchAreaRepo.Count() != 0 {
			t.Errorf("Expected empty repository, got count %d", searchAreaRepo.Count())
		}

		// Add a search area
		err = searchAreaRepo.Set("test-area", &models.SearchArea{
			Name:    "Test Area",
			LeftX:   100,
			TopY:    200,
			RightX:  300,
			BottomY: 400,
		})
		if err != nil {
			t.Fatalf("Failed to set search area: %v", err)
		}

		// Access the same repository again (should return the same instance)
		searchAreaRepo2 := retrieved.SearchAreaRepo("2560x1440")
		if searchAreaRepo2.Count() != 1 {
			t.Errorf("Expected count 1 from cached repository, got %d", searchAreaRepo2.Count())
		}

		// Verify the search area exists
		area, err := searchAreaRepo2.Get("test-area")
		if err != nil {
			t.Fatalf("Failed to get search area from cached repository: %v", err)
		}
		if area.LeftX != 100 {
			t.Errorf("Expected LeftX 100, got %d", area.LeftX)
		}
	})
}

// TestIntegration_CoordinateRepositories_MultipleResolutions tests that multiple resolution keys work independently
func TestIntegration_CoordinateRepositories_MultipleResolutions(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("Multiple resolutions for PointRepo", func(t *testing.T) {
		// Create a program
		program := models.NewProgram()
		program.Name = "Multi Resolution Test"

		// Save the program
		err := ProgramRepo().Set("multirestest", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program back
		retrieved, err := ProgramRepo().Get("multirestest")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Create repositories for different resolutions
		repo1080 := retrieved.PointRepo("1920x1080")
		repo1440 := retrieved.PointRepo("2560x1440")
		repo4k := retrieved.PointRepo("3840x2160")

		// Add points to each resolution
		err = repo1080.Set("point1", &models.Point{Name: "Point 1080", X: 100, Y: 100})
		if err != nil {
			t.Fatalf("Failed to set point in 1080 repo: %v", err)
		}

		err = repo1440.Set("point1", &models.Point{Name: "Point 1440", X: 200, Y: 200})
		if err != nil {
			t.Fatalf("Failed to set point in 1440 repo: %v", err)
		}

		err = repo4k.Set("point1", &models.Point{Name: "Point 4K", X: 300, Y: 300})
		if err != nil {
			t.Fatalf("Failed to set point in 4K repo: %v", err)
		}

		// Verify each repository has independent data
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

		point4k, err := repo4k.Get("point1")
		if err != nil {
			t.Fatalf("Failed to get point from 4K repo: %v", err)
		}
		if point4k.X != 300 {
			t.Errorf("Expected X 300 for 4K repo, got %d", point4k.X)
		}

		// Verify counts are independent
		if repo1080.Count() != 1 {
			t.Errorf("Expected count 1 for 1080 repo, got %d", repo1080.Count())
		}
		if repo1440.Count() != 1 {
			t.Errorf("Expected count 1 for 1440 repo, got %d", repo1440.Count())
		}
		if repo4k.Count() != 1 {
			t.Errorf("Expected count 1 for 4K repo, got %d", repo4k.Count())
		}
	})

	t.Run("Multiple resolutions for SearchAreaRepo", func(t *testing.T) {
		// Create a program
		program := models.NewProgram()
		program.Name = "Multi Resolution Search Area Test"

		// Save the program
		err := ProgramRepo().Set("multiressa", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program back
		retrieved, err := ProgramRepo().Get("multiressa")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Create repositories for different resolutions
		repo1080 := retrieved.SearchAreaRepo("1920x1080")
		repo1440 := retrieved.SearchAreaRepo("2560x1440")

		// Add search areas to each resolution
		err = repo1080.Set("area1", &models.SearchArea{
			Name:    "Area 1080",
			LeftX:   100,
			TopY:    100,
			RightX:  200,
			BottomY: 200,
		})
		if err != nil {
			t.Fatalf("Failed to set search area in 1080 repo: %v", err)
		}

		err = repo1440.Set("area1", &models.SearchArea{
			Name:    "Area 1440",
			LeftX:   200,
			TopY:    200,
			RightX:  400,
			BottomY: 400,
		})
		if err != nil {
			t.Fatalf("Failed to set search area in 1440 repo: %v", err)
		}

		// Verify each repository has independent data
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
	})
}

// TestIntegration_CoordinateRepositories_SavePersistence tests that Save() persists through ProgramRepository
func TestIntegration_CoordinateRepositories_SavePersistence(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("PointRepo Save persists through ProgramRepository", func(t *testing.T) {
		// Create and save a program
		program := models.NewProgram()
		program.Name = "Point Persistence Test"

		err := ProgramRepo().Set("pointpersist", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program and add points
		retrieved, err := ProgramRepo().Get("pointpersist")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		pointRepo := retrieved.PointRepo("1920x1080")
		err = pointRepo.Set("persist-point", &models.Point{Name: "Persist Point", X: 500, Y: 600})
		if err != nil {
			t.Fatalf("Failed to set point: %v", err)
		}

		// Reload the program repository (will re-read config from disk in test mode)
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		// Get the program again
		reloaded, err := ProgramRepo().Get("pointpersist")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		// Verify the point persisted
		reloadedPointRepo := reloaded.PointRepo("1920x1080")
		point, err := reloadedPointRepo.Get("persist-point")
		if err != nil {
			t.Fatalf("Point should exist after reload: %v", err)
		}

		if point.Name != "Persist Point" {
			t.Errorf("Expected name 'Persist Point', got '%s'", point.Name)
		}
		if point.X != 500 {
			t.Errorf("Expected X 500, got %d", point.X)
		}
		if point.Y != 600 {
			t.Errorf("Expected Y 600, got %d", point.Y)
		}
	})

	t.Run("SearchAreaRepo Save persists through ProgramRepository", func(t *testing.T) {
		// Create and save a program
		program := models.NewProgram()
		program.Name = "Search Area Persistence Test"

		err := ProgramRepo().Set("areapersist", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program and add search areas
		retrieved, err := ProgramRepo().Get("areapersist")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		searchAreaRepo := retrieved.SearchAreaRepo("2560x1440")
		err = searchAreaRepo.Set("persist-area", &models.SearchArea{
			Name:    "Persist Area",
			LeftX:   100,
			TopY:    200,
			RightX:  300,
			BottomY: 400,
		})
		if err != nil {
			t.Fatalf("Failed to set search area: %v", err)
		}

		// Reload the program repository (will re-read config from disk in test mode)
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		// Get the program again
		reloaded, err := ProgramRepo().Get("areapersist")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		// Verify the search area persisted
		reloadedSearchAreaRepo := reloaded.SearchAreaRepo("2560x1440")
		area, err := reloadedSearchAreaRepo.Get("persist-area")
		if err != nil {
			t.Fatalf("Search area should exist after reload: %v", err)
		}

		if area.Name != "Persist Area" {
			t.Errorf("Expected name 'Persist Area', got '%s'", area.Name)
		}
		if area.LeftX != 100 {
			t.Errorf("Expected LeftX 100, got %d", area.LeftX)
		}
		if area.RightX != 300 {
			t.Errorf("Expected RightX 300, got %d", area.RightX)
		}
	})

	t.Run("Multiple coordinate changes persist together", func(t *testing.T) {
		// Create and save a program
		program := models.NewProgram()
		program.Name = "Multiple Changes Test"

		err := ProgramRepo().Set("multichanges", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program
		retrieved, err := ProgramRepo().Get("multichanges")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Add multiple points and search areas
		pointRepo := retrieved.PointRepo("1920x1080")
		err = pointRepo.Set("point1", &models.Point{Name: "Point 1", X: 10, Y: 20})
		if err != nil {
			t.Fatalf("Failed to set point1: %v", err)
		}
		err = pointRepo.Set("point2", &models.Point{Name: "Point 2", X: 30, Y: 40})
		if err != nil {
			t.Fatalf("Failed to set point2: %v", err)
		}

		searchAreaRepo := retrieved.SearchAreaRepo("1920x1080")
		err = searchAreaRepo.Set("area1", &models.SearchArea{
			Name:    "Area 1",
			LeftX:   50,
			TopY:    60,
			RightX:  70,
			BottomY: 80,
		})
		if err != nil {
			t.Fatalf("Failed to set area1: %v", err)
		}

		// Reload
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		// Verify all changes persisted
		reloaded, err := ProgramRepo().Get("multichanges")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		reloadedPointRepo := reloaded.PointRepo("1920x1080")
		if reloadedPointRepo.Count() != 2 {
			t.Errorf("Expected 2 points after reload, got %d", reloadedPointRepo.Count())
		}

		reloadedSearchAreaRepo := reloaded.SearchAreaRepo("1920x1080")
		if reloadedSearchAreaRepo.Count() != 1 {
			t.Errorf("Expected 1 search area after reload, got %d", reloadedSearchAreaRepo.Count())
		}
	})
}

// TestIntegration_CoordinateRepositories_BackwardCompatibility tests backward compatibility with direct Coordinates map access
func TestIntegration_CoordinateRepositories_BackwardCompatibility(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("Direct Coordinates map access still works", func(t *testing.T) {
		// Create a program with coordinates using direct map access
		program := models.NewProgram()
		program.Name = "Backward Compatibility Test"

		// Add coordinates directly to the map (old way)
		program.Coordinates["1920x1080"] = &models.Coordinates{
			Points: map[string]*models.Point{
				"direct-point": {
					Name: "Direct Point",
					X:    100,
					Y:    200,
				},
			},
			SearchAreas: map[string]*models.SearchArea{
				"direct-area": {
					Name:    "Direct Area",
					LeftX:   10,
					TopY:    20,
					RightX:  30,
					BottomY: 40,
				},
			},
		}

		// Save the program
		err := ProgramRepo().Set("backcompat", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Reload
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		// Get the program and access via repository (new way)
		retrieved, err := ProgramRepo().Get("backcompat")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Verify repository can access directly-added coordinates
		pointRepo := retrieved.PointRepo("1920x1080")
		point, err := pointRepo.Get("direct-point")
		if err != nil {
			t.Fatalf("Repository should be able to access directly-added point: %v", err)
		}
		if point.X != 100 {
			t.Errorf("Expected X 100, got %d", point.X)
		}

		searchAreaRepo := retrieved.SearchAreaRepo("1920x1080")
		area, err := searchAreaRepo.Get("direct-area")
		if err != nil {
			t.Fatalf("Repository should be able to access directly-added search area: %v", err)
		}
		if area.LeftX != 10 {
			t.Errorf("Expected LeftX 10, got %d", area.LeftX)
		}
	})

	t.Run("Repository changes visible via direct map access", func(t *testing.T) {
		// Create a program
		program := models.NewProgram()
		program.Name = "Repo to Direct Test"

		err := ProgramRepo().Set("repotodirect", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program
		retrieved, err := ProgramRepo().Get("repotodirect")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Add coordinates via repository
		pointRepo := retrieved.PointRepo("2560x1440")
		err = pointRepo.Set("repo-point", &models.Point{Name: "Repo Point", X: 300, Y: 400})
		if err != nil {
			t.Fatalf("Failed to set point via repository: %v", err)
		}

		// Verify it's accessible via direct map access
		coords := retrieved.Coordinates["2560x1440"]
		if coords == nil {
			t.Fatal("Coordinates map should not be nil")
		}

		directPoint, exists := coords.Points["repo-point"]
		if !exists {
			t.Fatal("Point added via repository should be accessible via direct map")
		}
		if directPoint.X != 300 {
			t.Errorf("Expected X 300, got %d", directPoint.X)
		}
	})

	t.Run("Mixed access patterns work together", func(t *testing.T) {
		// Create a program
		program := models.NewProgram()
		program.Name = "Mixed Access Test"

		// Add some coordinates directly
		program.Coordinates["1920x1080"] = &models.Coordinates{
			Points: map[string]*models.Point{
				"direct1": {Name: "Direct 1", X: 10, Y: 20},
			},
			SearchAreas: make(map[string]*models.SearchArea),
		}

		err := ProgramRepo().Set("mixedaccess", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program
		retrieved, err := ProgramRepo().Get("mixedaccess")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Add more coordinates via repository
		pointRepo := retrieved.PointRepo("1920x1080")
		err = pointRepo.Set("repo1", &models.Point{Name: "Repo 1", X: 30, Y: 40})
		if err != nil {
			t.Fatalf("Failed to set point via repository: %v", err)
		}

		// Verify both are accessible
		if pointRepo.Count() != 2 {
			t.Errorf("Expected 2 points (1 direct + 1 repo), got %d", pointRepo.Count())
		}

		// Verify via direct access
		coords := retrieved.Coordinates["1920x1080"]
		if len(coords.Points) != 2 {
			t.Errorf("Expected 2 points in direct map, got %d", len(coords.Points))
		}

		// Verify both points exist
		_, err = pointRepo.Get("direct1")
		if err != nil {
			t.Error("Should be able to get directly-added point via repository")
		}

		_, err = pointRepo.Get("repo1")
		if err != nil {
			t.Error("Should be able to get repository-added point via repository")
		}
	})
}

// TestIntegration_CoordinateRepositories_ConfigFileVerification tests that changes are written to config file
func TestIntegration_CoordinateRepositories_ConfigFileVerification(t *testing.T) {
	configPath, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("Coordinate changes written to config file", func(t *testing.T) {
		// Create a program with coordinates
		program := models.NewProgram()
		program.Name = "Config File Test"

		err := ProgramRepo().Set("configfiletest", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program and add coordinates
		retrieved, err := ProgramRepo().Get("configfiletest")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		pointRepo := retrieved.PointRepo("1920x1080")
		err = pointRepo.Set("file-point", &models.Point{Name: "File Point", X: 777, Y: 888})
		if err != nil {
			t.Fatalf("Failed to set point: %v", err)
		}

		searchAreaRepo := retrieved.SearchAreaRepo("1920x1080")
		err = searchAreaRepo.Set("file-area", &models.SearchArea{
			Name:    "File Area",
			LeftX:   111,
			TopY:    222,
			RightX:  333,
			BottomY: 444,
		})
		if err != nil {
			t.Fatalf("Failed to set search area: %v", err)
		}

		// Read the config file directly
		content, err := os.ReadFile(configPath)
		if err != nil {
			t.Fatalf("Failed to read config file: %v", err)
		}

		configStr := string(content)

		// Verify the coordinates are in the file
		if !contains(configStr, "file-point") {
			t.Error("Config file should contain 'file-point'")
		}
		if !contains(configStr, "File Point") {
			t.Error("Config file should contain 'File Point'")
		}
		if !contains(configStr, "file-area") {
			t.Error("Config file should contain 'file-area'")
		}
		if !contains(configStr, "File Area") {
			t.Error("Config file should contain 'File Area'")
		}

		// Verify the coordinate values are in the file
		if !contains(configStr, "777") {
			t.Error("Config file should contain X coordinate 777")
		}
		if !contains(configStr, "888") {
			t.Error("Config file should contain Y coordinate 888")
		}
		if !contains(configStr, "111") {
			t.Error("Config file should contain LeftX coordinate 111")
		}
		if !contains(configStr, "333") {
			t.Error("Config file should contain RightX coordinate 333")
		}
	})

	t.Run("Multiple resolutions written to config file", func(t *testing.T) {
		// Create a program
		program := models.NewProgram()
		program.Name = "Multi Res Config Test"

		err := ProgramRepo().Set("multiresconfig", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get the program
		retrieved, err := ProgramRepo().Get("multiresconfig")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Add coordinates for multiple resolutions
		repo1080 := retrieved.PointRepo("1920x1080")
		err = repo1080.Set("point-1080", &models.Point{Name: "Point 1080", X: 1920, Y: 1080})
		if err != nil {
			t.Fatalf("Failed to set 1080 point: %v", err)
		}

		repo1440 := retrieved.PointRepo("2560x1440")
		err = repo1440.Set("point-1440", &models.Point{Name: "Point 1440", X: 2560, Y: 1440})
		if err != nil {
			t.Fatalf("Failed to set 1440 point: %v", err)
		}

		// Read the config file
		content, err := os.ReadFile(configPath)
		if err != nil {
			t.Fatalf("Failed to read config file: %v", err)
		}

		configStr := string(content)

		// Verify both resolutions are in the file
		if !contains(configStr, "1920x1080") {
			t.Error("Config file should contain resolution key '1920x1080'")
		}
		if !contains(configStr, "2560x1440") {
			t.Error("Config file should contain resolution key '2560x1440'")
		}
		if !contains(configStr, "point-1080") {
			t.Error("Config file should contain 'point-1080'")
		}
		if !contains(configStr, "point-1440") {
			t.Error("Config file should contain 'point-1440'")
		}
	})
}
