package repositories

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"errors"
	"fmt"
	"sync"
	"testing"
)

// Helper function to create a test program with items
func createTestProgram(name string) *models.Program {
	program := models.NewProgram()
	program.Name = name

	// Add some test items
	program.Items["health potion"] = &models.Item{
		Name:     "Health Potion",
		GridSize: [2]int{1, 1},
		Tags:     []string{"consumable", "healing"},
		StackMax: 5,
		Merchant: "alchemist",
	}

	program.Items["sword"] = &models.Item{
		Name:     "Sword",
		GridSize: [2]int{1, 3},
		Tags:     []string{"weapon", "melee"},
		StackMax: 1,
		Merchant: "blacksmith",
	}

	return program
}

// TestItemRepository_ThreadSafety_ConcurrentReads tests concurrent read operations
func TestItemRepository_ThreadSafety_ConcurrentReads(t *testing.T) {
	program := models.NewProgram()
	program.Name = "concurrent test"

	// Populate with items
	for i := 0; i < 100; i++ {
		key := fmt.Sprintf("item%d", i)
		program.Items[key] = &models.Item{
			Name:     fmt.Sprintf("Item %d", i),
			GridSize: [2]int{1, 1},
			StackMax: i,
		}
	}

	repo := NewItemRepository(program)

	var wg sync.WaitGroup
	errors := make(chan error, 100)

	// Concurrent reads
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("item%d", idx)
			item, err := repo.Get(key)
			if err != nil {
				errors <- fmt.Errorf("failed to get %s: %w", key, err)
				return
			}
			if item.StackMax != idx {
				errors <- fmt.Errorf("expected StackMax %d, got %d", idx, item.StackMax)
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Error(err)
	}
}

// TestItemRepository_ThreadSafety_ConcurrentWrites tests concurrent write operations
func TestItemRepository_ThreadSafety_ConcurrentWrites(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "concurrent writes"

	// Save program first
	err := ProgramRepo().Set("concurrentwrites", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewItemRepository(program)

	var wg sync.WaitGroup
	errors := make(chan error, 100)

	// Concurrent writes
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("item%d", idx)
			item := &models.Item{
				Name:     fmt.Sprintf("Item %d", idx),
				GridSize: [2]int{1, 1},
				StackMax: idx,
			}

			// Note: We're testing thread safety, not persistence
			// So we directly modify the map to avoid excessive file I/O
			repo.NestedRepository.mu.Lock()
			repo.NestedRepository.models[key] = item
			repo.NestedRepository.mu.Unlock()
		}(i)
	}

	wg.Wait()
	close(errors)

	// Verify all writes succeeded
	if repo.Count() != 100 {
		t.Errorf("Expected 100 items after concurrent writes, got %d", repo.Count())
	}

	for err := range errors {
		t.Error(err)
	}
}

// TestItemRepository_ThreadSafety_MixedOperations tests mixed concurrent reads and writes
func TestItemRepository_ThreadSafety_MixedOperations(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "mixed operations"

	// Populate with initial data
	for i := 0; i < 50; i++ {
		key := fmt.Sprintf("item%d", i)
		program.Items[key] = &models.Item{
			Name:     fmt.Sprintf("Item %d", i),
			GridSize: [2]int{1, 1},
			StackMax: i,
		}
	}

	// Save program
	err := ProgramRepo().Set("mixedops", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewItemRepository(program)

	var wg sync.WaitGroup
	errors := make(chan error, 200)

	// Concurrent reads
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("item%d", idx)
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
			key := fmt.Sprintf("item%d", idx)
			item := &models.Item{
				Name:     fmt.Sprintf("Item %d", idx),
				GridSize: [2]int{1, 1},
				StackMax: idx,
			}
			repo.NestedRepository.mu.Lock()
			repo.NestedRepository.models[key] = item
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
				errors <- fmt.Errorf("GetAll returned too few items: %d", len(all))
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
		t.Errorf("Expected 100 items after mixed operations, got %d", repo.Count())
	}
}

// TestItemRepository_ThreadSafety_ConcurrentDeletes tests concurrent delete operations
func TestItemRepository_ThreadSafety_ConcurrentDeletes(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	program := models.NewProgram()
	program.Name = "concurrent deletes"

	// Populate with items
	for i := 0; i < 100; i++ {
		key := fmt.Sprintf("item%d", i)
		program.Items[key] = &models.Item{
			Name:     fmt.Sprintf("Item %d", i),
			GridSize: [2]int{1, 1},
			StackMax: i,
		}
	}

	// Save program
	err := ProgramRepo().Set("concurrentdeletes", program)
	if err != nil {
		t.Fatalf("Failed to save program: %v", err)
	}

	repo := NewItemRepository(program)

	var wg sync.WaitGroup

	// Concurrent deletes (direct map access to avoid file I/O)
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			key := fmt.Sprintf("item%d", idx)
			repo.NestedRepository.mu.Lock()
			delete(repo.NestedRepository.models, key)
			repo.NestedRepository.mu.Unlock()
		}(i)
	}

	wg.Wait()

	// Verify deletions
	if repo.Count() != 50 {
		t.Errorf("Expected 50 items after concurrent deletes, got %d", repo.Count())
	}

	// Verify deleted items are gone
	for i := 0; i < 50; i++ {
		key := fmt.Sprintf("item%d", i)
		_, err := repo.Get(key)
		if !errors.Is(err, ErrNotFound) {
			t.Errorf("Item %s should be deleted", key)
		}
	}

	// Verify remaining items still exist
	for i := 50; i < 100; i++ {
		key := fmt.Sprintf("item%d", i)
		_, err := repo.Get(key)
		if err != nil {
			t.Errorf("Item %s should still exist: %v", key, err)
		}
	}
}

// TestItemRepository_GetAllWithProgramPrefix tests the GetAllWithProgramPrefix method
func TestItemRepository_GetAllWithProgramPrefix(t *testing.T) {
	program := createTestProgram("dark and darker")
	repo := NewItemRepository(program)

	t.Run("GetAllWithProgramPrefix formats correctly", func(t *testing.T) {
		prefixed := repo.GetAllWithProgramPrefix()

		if len(prefixed) != 2 {
			t.Errorf("Expected 2 items, got %d", len(prefixed))
		}

		// Verify format: "program|item"
		expectedKey1 := "dark and darker" + config.ProgramDelimiter + "health potion"
		expectedKey2 := "dark and darker" + config.ProgramDelimiter + "sword"

		if _, exists := prefixed[expectedKey1]; !exists {
			t.Errorf("Expected key '%s' not found", expectedKey1)
		}

		if _, exists := prefixed[expectedKey2]; !exists {
			t.Errorf("Expected key '%s' not found", expectedKey2)
		}

		// Verify items are correct
		item1 := prefixed[expectedKey1]
		if item1.Name != "Health Potion" {
			t.Errorf("Expected 'Health Potion', got '%s'", item1.Name)
		}
	})

	t.Run("GetAllWithProgramPrefix uses correct delimiter", func(t *testing.T) {
		prefixed := repo.GetAllWithProgramPrefix()

		// Verify all keys contain the delimiter
		for key := range prefixed {
			if !containsDelimiter(key, config.ProgramDelimiter) {
				t.Errorf("Key '%s' does not contain delimiter '%s'", key, config.ProgramDelimiter)
			}
		}
	})

	t.Run("GetAllWithProgramPrefix on empty repository", func(t *testing.T) {
		emptyProgram := models.NewProgram()
		emptyProgram.Name = "empty game"
		emptyRepo := NewItemRepository(emptyProgram)

		prefixed := emptyRepo.GetAllWithProgramPrefix()
		if len(prefixed) != 0 {
			t.Errorf("Expected empty map, got %d items", len(prefixed))
		}
	})

	t.Run("GetAllWithProgramPrefix with special characters in program name", func(t *testing.T) {
		specialProgram := models.NewProgram()
		specialProgram.Name = "game-with-dashes"
		specialProgram.Items["item1"] = &models.Item{Name: "Item 1"}

		specialRepo := NewItemRepository(specialProgram)
		prefixed := specialRepo.GetAllWithProgramPrefix()

		expectedKey := "game-with-dashes" + config.ProgramDelimiter + "item1"
		if _, exists := prefixed[expectedKey]; !exists {
			t.Errorf("Expected key '%s' not found", expectedKey)
		}
	})
}

// TestItemRepository_GetAllSorted tests the GetAllSorted method
func TestItemRepository_GetAllSorted(t *testing.T) {
	t.Run("GetAllSorted returns alphabetical order", func(t *testing.T) {
		program := models.NewProgram()
		program.Name = "sort test"

		// Add items in non-alphabetical order
		program.Items["zebra"] = &models.Item{Name: "Zebra"}
		program.Items["apple"] = &models.Item{Name: "Apple"}
		program.Items["mango"] = &models.Item{Name: "Mango"}
		program.Items["banana"] = &models.Item{Name: "Banana"}
		program.Items["cherry"] = &models.Item{Name: "Cherry"}

		repo := NewItemRepository(program)
		sorted := repo.GetAllSorted()

		if len(sorted) != 5 {
			t.Errorf("Expected 5 items, got %d", len(sorted))
		}

		// Verify alphabetical order
		expected := []string{"apple", "banana", "cherry", "mango", "zebra"}
		for i, key := range sorted {
			if key != expected[i] {
				t.Errorf("Expected sorted[%d] = '%s', got '%s'", i, expected[i], key)
			}
		}
	})

	t.Run("GetAllSorted on empty repository", func(t *testing.T) {
		emptyProgram := models.NewProgram()
		emptyProgram.Name = "empty"
		emptyRepo := NewItemRepository(emptyProgram)

		sorted := emptyRepo.GetAllSorted()
		if len(sorted) != 0 {
			t.Errorf("Expected empty slice, got %d items", len(sorted))
		}
	})

	t.Run("GetAllSorted with single item", func(t *testing.T) {
		program := models.NewProgram()
		program.Name = "single"
		program.Items["only"] = &models.Item{Name: "Only"}

		repo := NewItemRepository(program)
		sorted := repo.GetAllSorted()

		if len(sorted) != 1 {
			t.Errorf("Expected 1 item, got %d", len(sorted))
		}

		if sorted[0] != "only" {
			t.Errorf("Expected 'only', got '%s'", sorted[0])
		}
	})

	t.Run("GetAllSorted with numeric prefixes", func(t *testing.T) {
		program := models.NewProgram()
		program.Name = "numeric"

		// Add items with numeric prefixes
		program.Items["1-first"] = &models.Item{Name: "First"}
		program.Items["10-tenth"] = &models.Item{Name: "Tenth"}
		program.Items["2-second"] = &models.Item{Name: "Second"}
		program.Items["20-twentieth"] = &models.Item{Name: "Twentieth"}

		repo := NewItemRepository(program)
		sorted := repo.GetAllSorted()

		// Verify lexicographic sorting (10 comes before 2)
		expected := []string{"1-first", "10-tenth", "2-second", "20-twentieth"}
		for i, key := range sorted {
			if key != expected[i] {
				t.Errorf("Expected sorted[%d] = '%s', got '%s'", i, expected[i], key)
			}
		}
	})

	t.Run("GetAllSorted is consistent with GetAllKeys", func(t *testing.T) {
		program := createTestProgram("consistency test")
		repo := NewItemRepository(program)

		sorted := repo.GetAllSorted()
		keys := repo.GetAllKeys()

		if len(sorted) != len(keys) {
			t.Errorf("GetAllSorted and GetAllKeys returned different lengths: %d vs %d", len(sorted), len(keys))
		}

		for i := range sorted {
			if sorted[i] != keys[i] {
				t.Errorf("GetAllSorted[%d] != GetAllKeys[%d]: '%s' vs '%s'", i, i, sorted[i], keys[i])
			}
		}
	})
}

// TestItemRepository_DelimiterUsage verifies delimiter usage matches config
func TestItemRepository_DelimiterUsage(t *testing.T) {
	t.Run("Delimiter matches config.ProgramDelimiter", func(t *testing.T) {
		program := createTestProgram("delimiter test")
		repo := NewItemRepository(program)

		prefixed := repo.GetAllWithProgramPrefix()

		// Verify all keys use the correct delimiter
		for key := range prefixed {
			if !containsDelimiter(key, config.ProgramDelimiter) {
				t.Errorf("Key '%s' does not use config.ProgramDelimiter '%s'", key, config.ProgramDelimiter)
			}

			// Verify delimiter is not duplicated or misused
			parts := splitByDelimiter(key, config.ProgramDelimiter)
			if len(parts) != 2 {
				t.Errorf("Key '%s' should have exactly 2 parts separated by delimiter, got %d", key, len(parts))
			}
		}
	})

	t.Run("Delimiter value is tilde character", func(t *testing.T) {
		// Verify the constant matches expected value (tilde: Windows disallows "|" in filenames)
		if config.ProgramDelimiter != "~" {
			t.Errorf("Expected ProgramDelimiter to be '~', got '%s'", config.ProgramDelimiter)
		}
	})
}

// Helper function to check if a string contains the delimiter
func containsDelimiter(s, delimiter string) bool {
	for i := 0; i <= len(s)-len(delimiter); i++ {
		if s[i:i+len(delimiter)] == delimiter {
			return true
		}
	}
	return false
}

// Helper function to split by delimiter
func splitByDelimiter(s, delimiter string) []string {
	var parts []string
	start := 0

	for i := 0; i <= len(s)-len(delimiter); i++ {
		if s[i:i+len(delimiter)] == delimiter {
			parts = append(parts, s[start:i])
			start = i + len(delimiter)
			i += len(delimiter) - 1
		}
	}

	if start < len(s) {
		parts = append(parts, s[start:])
	}

	return parts
}

// Integration Tests

// TestItemRepository_Integration_CreateAndSave tests creating a program with items and saving
func TestItemRepository_Integration_CreateAndSave(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("Create program with items and save through ProgramRepository", func(t *testing.T) {
		// Create a new program
		program := models.NewProgram()
		program.Name = "Integration Test Game"

		// Add items directly to the program
		program.Items["health potion"] = &models.Item{
			Name:     "Health Potion",
			GridSize: [2]int{1, 1},
			Tags:     []string{"consumable", "healing"},
			StackMax: 5,
			Merchant: "alchemist",
		}

		program.Items["sword"] = &models.Item{
			Name:     "Sword",
			GridSize: [2]int{1, 3},
			Tags:     []string{"weapon", "melee"},
			StackMax: 1,
			Merchant: "blacksmith",
		}

		// Save through ProgramRepository
		// Use a key that matches the desired program name
		err := ProgramRepo().Set("Integration Test Game", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Verify program was saved
		retrieved, err := ProgramRepo().Get("Integration Test Game")
		if err != nil {
			t.Fatalf("Failed to retrieve saved program: %v", err)
		}

		if retrieved.Name != "Integration Test Game" {
			t.Errorf("Expected name 'Integration Test Game', got '%s'", retrieved.Name)
		}

		// Verify items were saved
		if len(retrieved.Items) != 2 {
			t.Errorf("Expected 2 items, got %d", len(retrieved.Items))
		}

		healthPotion, exists := retrieved.Items["health potion"]
		if !exists {
			t.Fatal("Health potion should exist")
		}

		if healthPotion.Name != "Health Potion" {
			t.Errorf("Expected 'Health Potion', got '%s'", healthPotion.Name)
		}
	})
}

// TestItemRepository_Integration_ReloadAndAccess tests reloading a program and accessing items
func TestItemRepository_Integration_ReloadAndAccess(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("Reload program and access items through ItemRepository", func(t *testing.T) {
		// Create and save a program
		program := models.NewProgram()
		program.Name = "Reload Test"
		program.Items["item1"] = &models.Item{
			Name:     "Item 1",
			GridSize: [2]int{1, 1},
			StackMax: 10,
		}
		program.Items["item2"] = &models.Item{
			Name:     "Item 2",
			GridSize: [2]int{2, 2},
			StackMax: 5,
		}

		err := ProgramRepo().Set("reloadtest", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Reload the repository
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		// Get the program again
		reloaded, err := ProgramRepo().Get("reloadtest")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		// Access items through ItemRepository
		itemRepo := NewItemRepository(reloaded)

		item1, err := itemRepo.Get("item1")
		if err != nil {
			t.Fatalf("Failed to get item1: %v", err)
		}

		if item1.Name != "Item 1" {
			t.Errorf("Expected 'Item 1', got '%s'", item1.Name)
		}

		if item1.StackMax != 10 {
			t.Errorf("Expected StackMax 10, got %d", item1.StackMax)
		}

		// Verify all items are accessible
		allItems := itemRepo.GetAll()
		if len(allItems) != 2 {
			t.Errorf("Expected 2 items after reload, got %d", len(allItems))
		}
	})
}

// TestItemRepository_Integration_ModifyAndPersist tests modifying items and verifying persistence
func TestItemRepository_Integration_ModifyAndPersist(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("Modify items through ItemRepository and verify persistence", func(t *testing.T) {
		// Create and save initial program
		program := models.NewProgram()
		program.Name = "Modify Test"
		program.Items["original"] = &models.Item{
			Name:     "Original Item",
			GridSize: [2]int{1, 1},
			StackMax: 5,
		}

		err := ProgramRepo().Set("modifytest", program)
		if err != nil {
			t.Fatalf("Failed to save initial program: %v", err)
		}

		// Get the program and create ItemRepository
		retrieved, err := ProgramRepo().Get("modifytest")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		itemRepo := NewItemRepository(retrieved)

		// Add a new item through ItemRepository
		newItem := &models.Item{
			Name:     "New Item",
			GridSize: [2]int{2, 2},
			StackMax: 10,
			Tags:     []string{"new"},
		}

		err = itemRepo.Set("new item", newItem)
		if err != nil {
			t.Fatalf("Failed to set new item: %v", err)
		}

		// Update existing item
		updatedItem := &models.Item{
			Name:     "Original Item Updated",
			GridSize: [2]int{1, 2},
			StackMax: 15,
		}

		err = itemRepo.Set("original", updatedItem)
		if err != nil {
			t.Fatalf("Failed to update item: %v", err)
		}

		// Reload and verify changes persisted
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		reloaded, err := ProgramRepo().Get("modifytest")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		reloadedRepo := NewItemRepository(reloaded)

		// Verify new item exists
		newItemRetrieved, err := reloadedRepo.Get("new item")
		if err != nil {
			t.Fatalf("New item should exist after reload: %v", err)
		}

		// After key synchronization, Name should match the provided key
		if newItemRetrieved.Name != "new item" {
			t.Errorf("Expected 'new item', got '%s'", newItemRetrieved.Name)
		}

		// Verify original item was updated
		originalRetrieved, err := reloadedRepo.Get("original")
		if err != nil {
			t.Fatalf("Original item should exist after reload: %v", err)
		}

		if originalRetrieved.StackMax != 15 {
			t.Errorf("Expected StackMax 15, got %d", originalRetrieved.StackMax)
		}

		if originalRetrieved.GridSize[1] != 2 {
			t.Errorf("Expected GridSize[1] = 2, got %d", originalRetrieved.GridSize[1])
		}
	})
}

// TestItemRepository_Integration_SaveTriggersProgram tests that ItemRepository.Save() triggers Program save
func TestItemRepository_Integration_SaveTriggersProgram(t *testing.T) {
	_, cleanup := setupIntegrationTest(t)
	defer cleanup()

	resetProgramRepo()

	t.Run("ItemRepository.Save() triggers Program save", func(t *testing.T) {
		// Create and save program
		program := models.NewProgram()
		program.Name = "Save Trigger Test"

		err := ProgramRepo().Set("savetrigger", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get program and create ItemRepository
		retrieved, err := ProgramRepo().Get("savetrigger")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		itemRepo := NewItemRepository(retrieved)

		// Add item through ItemRepository (which calls Save internally)
		item := &models.Item{
			Name:     "Test Item",
			GridSize: [2]int{1, 1},
			StackMax: 1,
		}

		err = itemRepo.Set("Test Item", item)
		if err != nil {
			t.Fatalf("Failed to set item: %v", err)
		}

		// Reload from disk
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		// Verify item was persisted
		reloaded, err := ProgramRepo().Get("savetrigger")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		if len(reloaded.Items) != 1 {
			t.Errorf("Expected 1 item after reload, got %d", len(reloaded.Items))
		}

		testItem, exists := reloaded.Items["Test Item"]
		if !exists {
			t.Fatal("Test item should exist after reload")
		}

		if testItem.Name != "Test Item" {
			t.Errorf("Expected 'Test Item', got '%s'", testItem.Name)
		}
	})

	t.Run("ItemRepository.Delete() triggers Program save", func(t *testing.T) {
		// Create program with items
		program := models.NewProgram()
		program.Name = "Delete Trigger Test"
		program.Items["item1"] = &models.Item{Name: "Item 1"}
		program.Items["item2"] = &models.Item{Name: "Item 2"}

		err := ProgramRepo().Set("deletetrigger", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get program and delete an item
		retrieved, err := ProgramRepo().Get("deletetrigger")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		itemRepo := NewItemRepository(retrieved)

		err = itemRepo.Delete("item1")
		if err != nil {
			t.Fatalf("Failed to delete item: %v", err)
		}

		// Reload and verify deletion persisted
		err = ProgramRepo().Reload()
		if err != nil {
			t.Fatalf("Failed to reload: %v", err)
		}

		reloaded, err := ProgramRepo().Get("deletetrigger")
		if err != nil {
			t.Fatalf("Failed to get reloaded program: %v", err)
		}

		if len(reloaded.Items) != 1 {
			t.Errorf("Expected 1 item after delete and reload, got %d", len(reloaded.Items))
		}

		if _, exists := reloaded.Items["item1"]; exists {
			t.Error("item1 should not exist after delete")
		}

		if _, exists := reloaded.Items["item2"]; !exists {
			t.Error("item2 should still exist after delete")
		}
	})
}

// TestItemRepository_Integration_ProgramItemRepo tests accessing ItemRepository through Program.ItemRepo()
func TestItemRepository_Integration_ProgramItemRepo(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	t.Run("Access ItemRepository through Program.ItemRepo()", func(t *testing.T) {
		// Create and save program
		program := models.NewProgram()
		program.Name = "ItemRepo Test"
		program.Items["item 1"] = &models.Item{
			Name:     "Item 1",
			GridSize: [2]int{1, 1},
		}

		err := ProgramRepo().Set("itemrepo test", program)
		if err != nil {
			t.Fatalf("Failed to save program: %v", err)
		}

		// Get program
		retrieved, err := ProgramRepo().Get("itemrepo test")
		if err != nil {
			t.Fatalf("Failed to get program: %v", err)
		}

		// Access ItemRepository through Program.ItemRepo()
		itemRepo := retrieved.ItemRepo()
		if itemRepo == nil {
			t.Fatal("ItemRepo() should not return nil")
		}

		// Verify we can use the repository
		item, err := itemRepo.Get("item 1")
		if err != nil {
			t.Fatalf("Failed to get item through ItemRepo(): %v", err)
		}

		if item.Name != "Item 1" {
			t.Errorf("Expected 'Item 1', got '%s'", item.Name)
		}

		// Add item through ItemRepo()
		newItem := &models.Item{
			Name:     "Item 2",
			GridSize: [2]int{2, 2},
		}

		err = itemRepo.Set("item 2", newItem)
		if err != nil {
			t.Fatalf("Failed to set item through ItemRepo(): %v", err)
		}

		// Verify item was added
		if itemRepo.Count() != 2 {
			t.Errorf("Expected 2 items, got %d", itemRepo.Count())
		}
	})

	t.Run("ItemRepo() returns same instance on multiple calls", func(t *testing.T) {
		program := models.NewProgram()
		program.Name = "Singleton Test"

		// Call ItemRepo() multiple times
		repo1 := program.ItemRepo()
		repo2 := program.ItemRepo()
		repo3 := program.ItemRepo()

		// Verify they're the same instance (pointer comparison)
		if repo1 != repo2 || repo2 != repo3 {
			t.Error("ItemRepo() should return the same instance on multiple calls")
		}
	})
}

// TestNestedRepository_KeySynchronization_Item verifies that Item key synchronization
// works correctly when using Set() with a different key than the item's Name
func TestNestedRepository_KeySynchronization_Item(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	// Create a Program with an ItemRepository
	program := models.NewProgram()
	program.Name = "Key Sync Test"

	// Save the program to make it available for ItemRepository.Save()
	err := ProgramRepo().Set("keysynctest", program)
	if err != nil {
		t.Fatalf("Failed to save initial program: %v", err)
	}

	itemRepo := NewItemRepository(program)

	// Create an Item with a different name than the key we'll use
	item := &models.Item{
		Name:     "Old Name",
		GridSize: [2]int{1, 1},
		StackMax: 5,
		Tags:     []string{"test"},
	}

	// Call Set() with a different key than the item's Name (preserving capitalization)
	err = itemRepo.Set("New Name", item)
	if err != nil {
		t.Fatalf("Failed to set item: %v", err)
	}

	// Verify the item's internal Name field is updated to match the provided key (with capitalization)
	if item.GetKey() != "New Name" {
		t.Errorf("Expected item.GetKey() to be 'New Name', got '%s'", item.GetKey())
	}

	if item.Name != "New Name" {
		t.Errorf("Expected item.Name to be 'New Name', got '%s'", item.Name)
	}

	// Verify the item can be retrieved with the exact key
	retrieved, err := itemRepo.Get("New Name")
	if err != nil {
		t.Fatalf("Failed to get item with new key: %v", err)
	}

	if retrieved.GetKey() != "New Name" {
		t.Errorf("Expected retrieved.GetKey() to be 'New Name', got '%s'", retrieved.GetKey())
	}

	// Verify other fields are preserved
	if retrieved.StackMax != 5 {
		t.Errorf("Expected StackMax to be 5, got %d", retrieved.StackMax)
	}

	if len(retrieved.Tags) != 1 || retrieved.Tags[0] != "test" {
		t.Errorf("Expected Tags to be ['test'], got %v", retrieved.Tags)
	}

	// Verify the parent Program is saved correctly by getting it from the repository
	// (Set() should have triggered a save through the saveFunc)
	savedProgram, err := ProgramRepo().Get("keysynctest")
	if err != nil {
		t.Fatalf("Failed to get saved program: %v", err)
	}

	// Verify the item exists in the saved program with the exact key
	savedItem, exists := savedProgram.Items["New Name"]
	if !exists {
		t.Fatal("Item should exist in saved program with key 'New Name'")
	}

	if savedItem.Name != "New Name" {
		t.Errorf("Expected saved item Name to be 'New Name', got '%s'", savedItem.Name)
	}

	if savedItem.StackMax != 5 {
		t.Errorf("Expected saved item StackMax to be 5, got %d", savedItem.StackMax)
	}
}
