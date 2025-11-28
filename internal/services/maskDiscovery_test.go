package services

import (
	"os"
	"path/filepath"
	"testing"
)

func TestMaskDiscoveryService_ScanMasksDirectory(t *testing.T) {
	// Create a temporary directory for testing
	tempDir, err := os.MkdirTemp("", "mask_test")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	// Create test directory structure
	program1Dir := filepath.Join(tempDir, "dark and darker")
	program2Dir := filepath.Join(tempDir, "path of exile 2")
	
	if err := os.MkdirAll(program1Dir, 0755); err != nil {
		t.Fatalf("Failed to create program1 directory: %v", err)
	}
	if err := os.MkdirAll(program2Dir, 0755); err != nil {
		t.Fatalf("Failed to create program2 directory: %v", err)
	}

	// Create test mask files
	testFiles := []struct {
		path string
		name string
	}{
		{filepath.Join(program1Dir, "mask1.png"), "mask1"},
		{filepath.Join(program1Dir, "mask2.jpg"), "mask2"},
		{filepath.Join(program1Dir, "mask3.jpeg"), "mask3"},
		{filepath.Join(program1Dir, "invalid.txt"), ""}, // Should be ignored
		{filepath.Join(program2Dir, "poe_mask.png"), "poe_mask"},
	}

	for _, tf := range testFiles {
		if tf.name != "" { // Only create valid image files
			file, err := os.Create(tf.path)
			if err != nil {
				t.Fatalf("Failed to create test file %s: %v", tf.path, err)
			}
			file.Close()
		}
	}

	// Create service with custom base path
	service := &MaskDiscoveryService{basePath: tempDir}

	// Test ScanMasksDirectory
	masksByProgram, err := service.ScanMasksDirectory()
	if err != nil {
		t.Fatalf("ScanMasksDirectory failed: %v", err)
	}

	// Verify results
	if len(masksByProgram) != 2 {
		t.Errorf("Expected 2 programs, got %d", len(masksByProgram))
	}

	// Check dark and darker program
	darkMasks, exists := masksByProgram["dark and darker"]
	if !exists {
		t.Error("Expected 'dark and darker' program not found")
	} else if len(darkMasks) != 3 {
		t.Errorf("Expected 3 masks for 'dark and darker', got %d", len(darkMasks))
	}

	// Check path of exile 2 program
	poeMasks, exists := masksByProgram["path of exile 2"]
	if !exists {
		t.Error("Expected 'path of exile 2' program not found")
	} else if len(poeMasks) != 1 {
		t.Errorf("Expected 1 mask for 'path of exile 2', got %d", len(poeMasks))
	}
}

func TestMaskDiscoveryService_FilterMasks(t *testing.T) {
	service := &MaskDiscoveryService{}
	
	masks := []MaskInfo{
		{Name: "health_potion", Program: "test", Path: "/test/health_potion.png", Format: ".png"},
		{Name: "mana_potion", Program: "test", Path: "/test/mana_potion.png", Format: ".png"},
		{Name: "sword", Program: "test", Path: "/test/sword.jpg", Format: ".jpg"},
		{Name: "shield", Program: "test", Path: "/test/shield.jpeg", Format: ".jpeg"},
	}

	// Test empty search term
	filtered := service.FilterMasks(masks, "")
	if len(filtered) != 4 {
		t.Errorf("Expected 4 masks with empty search, got %d", len(filtered))
	}

	// Test search for "potion"
	filtered = service.FilterMasks(masks, "potion")
	if len(filtered) != 2 {
		t.Errorf("Expected 2 masks with 'potion' search, got %d", len(filtered))
	}

	// Test case insensitive search
	filtered = service.FilterMasks(masks, "SWORD")
	if len(filtered) != 1 {
		t.Errorf("Expected 1 mask with 'SWORD' search, got %d", len(filtered))
	}

	// Test no matches
	filtered = service.FilterMasks(masks, "nonexistent")
	if len(filtered) != 0 {
		t.Errorf("Expected 0 masks with 'nonexistent' search, got %d", len(filtered))
	}
}

func TestMaskDiscoveryService_ValidateMaskFile(t *testing.T) {
	service := &MaskDiscoveryService{}

	// Test empty path
	err := service.ValidateMaskFile("")
	if err == nil {
		t.Error("Expected error for empty path")
	}

	// Test non-existent file
	err = service.ValidateMaskFile("/nonexistent/file.png")
	if err == nil {
		t.Error("Expected error for non-existent file")
	}

	// Create a temporary file for testing
	tempFile, err := os.CreateTemp("", "test_mask*.png")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tempFile.Name())
	tempFile.Close()

	// Test valid PNG extension
	err = service.ValidateMaskFile(tempFile.Name())
	if err != nil {
		t.Errorf("Expected no error for valid PNG file, got: %v", err)
	}

	// Test invalid extension
	invalidFile := tempFile.Name() + ".txt"
	os.Rename(tempFile.Name(), invalidFile)
	defer os.Remove(invalidFile)
	
	err = service.ValidateMaskFile(invalidFile)
	if err == nil {
		t.Error("Expected error for invalid file extension")
	}
}