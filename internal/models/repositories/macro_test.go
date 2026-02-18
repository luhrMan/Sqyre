package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/serialize"
	"errors"
	"os"
	"path/filepath"
	"sync"
	"testing"
)

// setupTestConfig initializes YAML config with test configuration
func setupTestConfig(t *testing.T) {
	t.Helper()

	// Get the path to testdata
	testdataPath, err := filepath.Abs("testdata")
	if err != nil {
		t.Fatalf("Failed to get testdata path: %v", err)
	}

	// Configure YAML config to use writeable test config
	yamlConfig := serialize.GetYAMLConfig()
	configPath := filepath.Join(testdataPath, "writeable-db.yaml")
	yamlConfig.SetConfigFile(configPath)

	// Read the base config first
	baseConfigPath := filepath.Join(testdataPath, "db.yaml")
	yamlConfig.SetConfigFile(baseConfigPath)
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("Failed to read test db: %v", err)
	}

	// Set to writeable config for tests
	yamlConfig.SetConfigFile(configPath)
	if err := yamlConfig.WriteConfig(); err != nil {
		t.Fatalf("Failed to write test db: %v", err)
	}
}

// resetMacroRepo resets the singleton for testing
func resetMacroRepo() {
	macroRepo = nil
	macroOnce = sync.Once{}
}

func TestMacroRepo_Singleton(t *testing.T) {
	setupTestConfig(t)
	resetMacroRepo()

	// Get repository twice
	repo1 := MacroRepo()
	repo2 := MacroRepo()

	// Verify they're the same instance
	if repo1 != repo2 {
		t.Error("MacroRepo() should return the same singleton instance")
	}
}

func TestMacroRepo_ConcurrentSingletonAccess(t *testing.T) {
	setupTestConfig(t)
	resetMacroRepo()

	var wg sync.WaitGroup
	repos := make([]*MacroRepository, 100)

	// Concurrent access to singleton
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			repos[idx] = MacroRepo()
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

func TestMacroRepo_LoadFromConfig(t *testing.T) {
	setupTestConfig(t)
	resetMacroRepo()

	repo := MacroRepo()

	// Verify macro was loaded
	if repo.Count() == 0 {
		t.Fatal("Expected macros to be loaded from config")
	}

	// Get the test macro
	macro, err := repo.Get("Test Macro")
	if err != nil {
		t.Fatalf("Failed to get test macro: %v", err)
	}

	// Verify macro properties
	if macro.Name != "Test Macro" {
		t.Errorf("Expected name 'Test Macro', got '%s'", macro.Name)
	}

	if macro.GlobalDelay != 100 {
		t.Errorf("Expected GlobalDelay 100, got %d", macro.GlobalDelay)
	}

	if len(macro.Hotkey) != 3 {
		t.Errorf("Expected 3 hotkey parts, got %d", len(macro.Hotkey))
	}

	if macro.Root == nil {
		t.Fatal("Root action should not be nil")
	}

	if macro.Root.Name != "root" {
		t.Errorf("Expected root name 'root', got '%s'", macro.Root.Name)
	}
}

func TestMacroRepo_DecodeWithActions(t *testing.T) {
	setupTestConfig(t)
	resetMacroRepo()

	repo := MacroRepo()

	macro, err := repo.Get("Test Macro")
	if err != nil {
		t.Fatalf("Failed to get test macro: %v", err)
	}

	// Verify root loop has subactions
	if macro.Root == nil {
		t.Fatal("Root should not be nil")
	}

	subActions := macro.Root.GetSubActions()
	if len(subActions) != 3 {
		t.Errorf("Expected 3 subactions, got %d", len(subActions))
	}

	// Verify action types
	if len(subActions) >= 1 && subActions[0].GetType() != "wait" {
		t.Errorf("Expected first action to be 'wait', got '%s'", subActions[0].GetType())
	}

	if len(subActions) >= 2 && subActions[1].GetType() != "click" {
		t.Errorf("Expected second action to be 'click', got '%s'", subActions[1].GetType())
	}

	if len(subActions) >= 3 && subActions[2].GetType() != "move" {
		t.Errorf("Expected third action to be 'move', got '%s'", subActions[2].GetType())
	}
}

func TestMacroRepo_New(t *testing.T) {
	setupTestConfig(t)
	resetMacroRepo()

	repo := MacroRepo()

	// Test New creates a valid macro
	newMacro := repo.New()
	if newMacro == nil {
		t.Fatal("New() returned nil")
	}

	if newMacro.Root == nil {
		t.Error("New macro should have a root loop")
	}

	if newMacro.Root.Name != "root" {
		t.Errorf("Expected root name 'root', got '%s'", newMacro.Root.Name)
	}
}

func TestDecodeMacro_InvalidKey(t *testing.T) {
	setupTestConfig(t)

	// Try to decode a non-existent macro
	// Note: Viper doesn't error on non-existent keys, it returns empty values
	// This test verifies the decode function can handle this gracefully
	macro, err := decodeMacro("nonexistent")
	if err != nil {
		t.Errorf("Unexpected error: %v", err)
	}

	// The macro will be decoded but with empty/zero values
	if macro == nil {
		t.Error("Expected non-nil macro even for non-existent key")
	}
}

func TestMacroRepo_Reload(t *testing.T) {
	setupTestConfig(t)
	resetMacroRepo()

	repo := MacroRepo()
	initialCount := repo.Count()

	// Add a macro directly to the map (simulating runtime addition)
	repo.mu.Lock()
	repo.models["runtime"] = models.NewMacro("Runtime Macro", 0, []string{})
	repo.mu.Unlock()

	if repo.Count() != initialCount+1 {
		t.Error("Failed to add runtime macro")
	}

	// Reload should reset to config state
	err := repo.Reload()
	if err != nil {
		t.Fatalf("Reload failed: %v", err)
	}

	// Runtime macro should be gone
	_, err = repo.Get("runtime")
	if !errors.Is(err, ErrNotFound) {
		t.Error("Runtime macro should not exist after reload")
	}

	// Original macro should still exist
	_, err = repo.Get("Test Macro")
	if err != nil {
		t.Errorf("Original macro should exist after reload: %v", err)
	}
}

// TestMain sets up and tears down test environment
func TestMain(m *testing.M) {
	// Run tests
	code := m.Run()

	// Cleanup
	os.Exit(code)
}
