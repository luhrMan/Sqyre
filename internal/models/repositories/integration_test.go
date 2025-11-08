package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
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
	tempDir, err := os.MkdirTemp("", "sqyre-integration-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}

	// Create config file path
	configPath := filepath.Join(tempDir, "config.yaml")

	// Write initial test config
	initialConfig := `macros:
  integrationmacro:
    name: "Integration Test Macro"
    globaldelay: 50
    hotkey:
      - "ctrl"
      - "i"
    root:
      type: "loop"
      name: "root"
      count: 1
      subactions:
        - type: "wait"
          time: 100

programs:
  integrationprogram:
    name: "Integration Test Program"
    items:
      integrationitem:
        name: "Integration Item"
        gridSize: [2, 2]
        tags: ["integration"]
        stackMax: 20
        merchant: "integration merchant"
    coordinates:
      1920x1080:
        points:
          integrationpoint:
            name: "Integration Point"
            x: 800
            y: 900
        searchareas:
          integrationarea:
            name: "Integration Area"
            leftx: 200
            topy: 200
            rightx: 600
            bottomy: 600
`

	if err := os.WriteFile(configPath, []byte(initialConfig), 0644); err != nil {
		os.RemoveAll(tempDir)
		t.Fatalf("Failed to write config file: %v", err)
	}

	// Configure Viper to use temp config
	viper := serialize.GetViper()
	viper.SetConfigFile(configPath)
	if err := viper.ReadInConfig(); err != nil {
		os.RemoveAll(tempDir)
		t.Fatalf("Failed to read config: %v", err)
	}

	// Cleanup function
	cleanup := func() {
		// Disable test mode
		os.Unsetenv("SQYRE_TEST_MODE")
		
		// Reset viper to testdata config for other tests
		testdataPath, _ := filepath.Abs("testdata")
		viper.AddConfigPath(testdataPath)
		viper.SetConfigName("config")
		viper.SetConfigType("yaml")
		viper.ReadInConfig()
		
		os.RemoveAll(tempDir)
	}

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
	if macroRepo.Count() != 1 {
		t.Fatalf("Expected 1 macro initially, got %d", macroRepo.Count())
	}
	if programRepo.Count() != 1 {
		t.Fatalf("Expected 1 program initially, got %d", programRepo.Count())
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
	if macroRepo.Count() != 2 {
		t.Errorf("Expected 2 macros after reload, got %d", macroRepo.Count())
	}
	if programRepo.Count() != 2 {
		t.Errorf("Expected 2 programs after reload, got %d", programRepo.Count())
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
			_, err := macroRepo.Get("integrationmacro")
			if err != nil {
				errors <- err
			}
		}()

		wg.Add(1)
		go func() {
			defer wg.Done()
			_, err := programRepo.Get("integrationprogram")
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
	macro, err := macroRepo.Get("integrationmacro")
	if err != nil {
		t.Fatalf("Failed to get macro: %v", err)
	}
	macro.GlobalDelay = 999
	
	macroRepo.mu.Lock()
	macroRepo.models["integrationmacro"] = macro
	macroRepo.mu.Unlock()

	// Modify program
	program, err := programRepo.Get("integrationprogram")
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
	programRepo.models["integrationprogram"] = program
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
	reloadedMacro, err := macroRepo.Get("integrationmacro")
	if err != nil {
		t.Fatalf("Failed to get macro after reload: %v", err)
	}
	if reloadedMacro.GlobalDelay != 999 {
		t.Errorf("Macro modification not persisted: expected GlobalDelay 999, got %d", reloadedMacro.GlobalDelay)
	}

	// Verify program modifications persisted
	reloadedProgram, err := programRepo.Get("integrationprogram")
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
	_, err := repo.Get("integrationmacro")
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
	program.Coordinates["1920x1080"] = &coordinates.Coordinates{
		Points: map[string]*coordinates.Point{
			"point1": {Name: "Point 1", X: 100, Y: 200},
			"point2": {Name: "Point 2", X: 300, Y: 400},
		},
		SearchAreas: map[string]*coordinates.SearchArea{
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
