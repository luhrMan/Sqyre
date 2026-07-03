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
	for i := range 100 {
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
