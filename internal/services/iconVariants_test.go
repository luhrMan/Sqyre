package services

import (
	"os"
	"path/filepath"
	"testing"
)

// createTestIconFile creates a valid PNG file for testing
func createTestIconFile(t *testing.T, path string) {
	t.Helper()

	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}

	// PNG signature: \x89PNG\r\n\x1a\n followed by minimal valid PNG data
	pngData := []byte{
		0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
		0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
		0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 pixel
		0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
		0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
		0x54, 0x08, 0x99, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
		0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
		0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
		0x44, 0xAE, 0x42, 0x60, 0x82, // IEND chunk
	}

	if err := os.WriteFile(path, pngData, 0644); err != nil {
		t.Fatalf("Failed to create test PNG file: %v", err)
	}
}

// createInvalidFile creates a non-PNG file for testing
func createInvalidFile(t *testing.T, path string) {
	t.Helper()

	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}

	if err := os.WriteFile(path, []byte("not a png file"), 0644); err != nil {
		t.Fatalf("Failed to create invalid file: %v", err)
	}
}

// TestGetBaseItemName tests the GetBaseItemName method
func TestGetBaseItemName(t *testing.T) {
	service := NewIconVariantService()

	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{
			name:     "Item with variant",
			input:    "Health Potion|Ice",
			expected: "Health Potion",
		},
		{
			name:     "Item without variant",
			input:    "Health Potion",
			expected: "Health Potion",
		},
		{
			name:     "Item with multiple delimiters",
			input:    "Health Potion|Ice|Blue",
			expected: "Health Potion",
		},
		{
			name:     "Empty string",
			input:    "",
			expected: "",
		},
		{
			name:     "Only delimiter",
			input:    "|",
			expected: "",
		},
		{
			name:     "Delimiter at end",
			input:    "Health Potion|",
			expected: "Health Potion",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := service.GetBaseItemName(tt.input)
			if result != tt.expected {
				t.Errorf("Expected '%s', got '%s'", tt.expected, result)
			}
		})
	}
}

// TestValidateVariantFile tests the ValidateVariantFile method
func TestValidateVariantFile(t *testing.T) {
	service := NewIconVariantService()

	// Create temporary directory for test files
	tempDir := t.TempDir()

	t.Run("Valid PNG file", func(t *testing.T) {
		validPNG := filepath.Join(tempDir, "valid.png")
		createTestIconFile(t, validPNG)

		err := service.ValidateVariantFile(validPNG)
		if err != nil {
			t.Errorf("Expected no error for valid PNG, got: %v", err)
		}
	})

	t.Run("Invalid PNG file (wrong header)", func(t *testing.T) {
		invalidPNG := filepath.Join(tempDir, "invalid.png")
		createInvalidFile(t, invalidPNG)

		err := service.ValidateVariantFile(invalidPNG)
		if err == nil {
			t.Error("Expected error for invalid PNG file")
		}
	})

	t.Run("Non-existent file", func(t *testing.T) {
		nonExistent := filepath.Join(tempDir, "nonexistent.png")

		err := service.ValidateVariantFile(nonExistent)
		if err == nil {
			t.Error("Expected error for non-existent file")
		}
	})

	t.Run("Empty path", func(t *testing.T) {
		err := service.ValidateVariantFile("")
		if err == nil {
			t.Error("Expected error for empty path")
		}
	})

	t.Run("Directory instead of file", func(t *testing.T) {
		dir := filepath.Join(tempDir, "testdir")
		if err := os.MkdirAll(dir, 0755); err != nil {
			t.Fatalf("Failed to create test directory: %v", err)
		}

		err := service.ValidateVariantFile(dir)
		if err == nil {
			t.Error("Expected error for directory path")
		}
	})

	t.Run("File too small", func(t *testing.T) {
		smallFile := filepath.Join(tempDir, "small.png")
		if err := os.WriteFile(smallFile, []byte{0x89, 0x50}, 0644); err != nil {
			t.Fatalf("Failed to create small file: %v", err)
		}

		err := service.ValidateVariantFile(smallFile)
		if err == nil {
			t.Error("Expected error for file too small")
		}
	})
}

// TestGetVariants tests the GetVariants method
func TestGetVariants(t *testing.T) {
	// Use temporary directory for testing
	tempDir := t.TempDir()
	service := &IconVariantService{basePath: filepath.Join(tempDir, "icons") + "/"}

	programName := "test-game"
	itemName := "Health Potion"
	iconsPath := filepath.Join(tempDir, "icons", programName)

	t.Run("No variants exist", func(t *testing.T) {
		variants, err := service.GetVariants(programName, itemName)
		if err != nil {
			t.Errorf("Expected no error, got: %v", err)
		}

		if len(variants) != 0 {
			t.Errorf("Expected 0 variants, got %d", len(variants))
		}
	})

	t.Run("Multiple variants exist", func(t *testing.T) {
		// Create test icon files
		createTestIconFile(t, filepath.Join(iconsPath, "Health Potion|Ice.png"))
		createTestIconFile(t, filepath.Join(iconsPath, "Health Potion|Fire.png"))
		createTestIconFile(t, filepath.Join(iconsPath, "Health Potion|Original.png"))

		variants, err := service.GetVariants(programName, itemName)
		if err != nil {
			t.Fatalf("Expected no error, got: %v", err)
		}

		if len(variants) != 3 {
			t.Errorf("Expected 3 variants, got %d", len(variants))
		}

		// Verify variants are sorted
		expected := []string{"Fire", "Ice", "Original"}
		for i, v := range variants {
			if v != expected[i] {
				t.Errorf("Expected variant[%d] = '%s', got '%s'", i, expected[i], v)
			}
		}
	})

	t.Run("Legacy icon without variant", func(t *testing.T) {
		// Create legacy icon (no delimiter)
		createTestIconFile(t, filepath.Join(iconsPath, "Mana Potion.png"))

		variants, err := service.GetVariants(programName, "Mana Potion")
		if err != nil {
			t.Fatalf("Expected no error, got: %v", err)
		}

		if len(variants) != 1 {
			t.Errorf("Expected 1 variant (legacy), got %d", len(variants))
		}

		if variants[0] != "" {
			t.Errorf("Expected empty string for legacy icon, got '%s'", variants[0])
		}
	})

	t.Run("Mixed legacy and variant icons", func(t *testing.T) {
		// Create both legacy and variant icons
		createTestIconFile(t, filepath.Join(iconsPath, "Sword.png"))
		createTestIconFile(t, filepath.Join(iconsPath, "Sword|Enchanted.png"))

		variants, err := service.GetVariants(programName, "Sword")
		if err != nil {
			t.Fatalf("Expected no error, got: %v", err)
		}

		if len(variants) != 2 {
			t.Errorf("Expected 2 variants, got %d", len(variants))
		}

		// Verify empty string (legacy) comes first when sorted
		if variants[0] != "" {
			t.Errorf("Expected first variant to be empty string (legacy), got '%s'", variants[0])
		}
		if variants[1] != "Enchanted" {
			t.Errorf("Expected second variant to be 'Enchanted', got '%s'", variants[1])
		}
	})

	t.Run("Empty program name", func(t *testing.T) {
		_, err := service.GetVariants("", itemName)
		if err == nil {
			t.Error("Expected error for empty program name")
		}
	})

	t.Run("Empty item name", func(t *testing.T) {
		_, err := service.GetVariants(programName, "")
		if err == nil {
			t.Error("Expected error for empty item name")
		}
	})
}

// TestGetVariantPath tests the GetVariantPath method
func TestGetVariantPath(t *testing.T) {
	tempDir := t.TempDir()
	service := &IconVariantService{basePath: filepath.Join(tempDir, "icons") + "/"}

	programName := "test-game"
	itemName := "Health Potion"

	t.Run("Variant with name", func(t *testing.T) {
		path := service.GetVariantPath(programName, itemName, "Ice")
		expected := filepath.Join(tempDir, "icons", programName, "Health Potion|Ice.png")

		if path != expected {
			t.Errorf("Expected path '%s', got '%s'", expected, path)
		}
	})

	t.Run("Legacy variant (empty name)", func(t *testing.T) {
		path := service.GetVariantPath(programName, itemName, "")
		expected := filepath.Join(tempDir, "icons", programName, "Health Potion.png")

		if path != expected {
			t.Errorf("Expected path '%s', got '%s'", expected, path)
		}
	})
}

// TestAddVariant tests the AddVariant method
func TestAddVariant(t *testing.T) {
	// Create temporary directory for testing
	tempDir := t.TempDir()
	sourceDir := filepath.Join(tempDir, "source")
	service := &IconVariantService{basePath: filepath.Join(tempDir, "icons") + "/"}

	programName := "test-game-add"
	itemName := "Health Potion"

	t.Run("Add valid variant", func(t *testing.T) {
		// Create source file
		sourcePath := filepath.Join(sourceDir, "test.png")
		createTestIconFile(t, sourcePath)

		// First variant will be forced to "Original"
		err := service.AddVariant(programName, itemName, "Ice", sourcePath)
		if err != nil {
			t.Fatalf("Expected no error, got: %v", err)
		}

		// Verify file was copied with "Original" name
		destPath := service.GetVariantPath(programName, itemName, "Original")
		if _, err := os.Stat(destPath); os.IsNotExist(err) {
			t.Error("Original variant file was not created")
		}

		// Verify it's a valid PNG
		if err := service.ValidateVariantFile(destPath); err != nil {
			t.Errorf("Copied file is not a valid PNG: %v", err)
		}
	})

	t.Run("Add duplicate variant", func(t *testing.T) {
		sourcePath := filepath.Join(sourceDir, "test2.png")
		createTestIconFile(t, sourcePath)

		// First add should succeed and become "Original"
		err := service.AddVariant(programName, itemName, "Fire", sourcePath)
		if err != nil {
			t.Fatalf("First add failed: %v", err)
		}

		// Second add with "Original" variant name should fail with VariantExistsError
		err = service.AddVariant(programName, itemName, "Original", sourcePath)
		if err == nil {
			t.Error("Expected error for duplicate variant name")
		}
		
		// Check that it's the correct error type
		if _, ok := err.(*VariantExistsError); !ok {
			t.Errorf("Expected VariantExistsError, got %T", err)
		}
	})

	t.Run("Add invalid PNG file", func(t *testing.T) {
		invalidPath := filepath.Join(sourceDir, "invalid.png")
		createInvalidFile(t, invalidPath)

		err := service.AddVariant(programName, itemName, "Invalid", invalidPath)
		if err == nil {
			t.Error("Expected error for invalid PNG file")
		}
	})

	t.Run("Empty parameters", func(t *testing.T) {
		sourcePath := filepath.Join(sourceDir, "test3.png")
		createTestIconFile(t, sourcePath)

		tests := []struct {
			name        string
			programName string
			itemName    string
			variantName string
			sourcePath  string
		}{
			{"Empty program name", "", itemName, "Test", sourcePath},
			{"Empty item name", programName, "", "Test", sourcePath},
			{"Empty variant name", programName, itemName, "", sourcePath},
			{"Empty source path", programName, itemName, "Test", ""},
		}

		for _, tt := range tests {
			t.Run(tt.name, func(t *testing.T) {
				err := service.AddVariant(tt.programName, tt.itemName, tt.variantName, tt.sourcePath)
				if err == nil {
					t.Error("Expected error for empty parameter")
				}
			})
		}
	})

	t.Run("Invalid variant name with path separators", func(t *testing.T) {
		sourcePath := filepath.Join(sourceDir, "test4.png")
		createTestIconFile(t, sourcePath)

		invalidNames := []string{"../test", "test/variant", "test\\variant", ".."}
		for _, name := range invalidNames {
			err := service.AddVariant(programName, itemName, name, sourcePath)
			if err == nil {
				t.Errorf("Expected error for invalid variant name '%s'", name)
			}
		}
	})

	t.Run("Non-existent source file", func(t *testing.T) {
		err := service.AddVariant(programName, itemName, "NonExistent", "/nonexistent/file.png")
		if err == nil {
			t.Error("Expected error for non-existent source file")
		}
	})
}

// TestDeleteVariant tests the DeleteVariant method
func TestDeleteVariant(t *testing.T) {
	tempDir := t.TempDir()
	service := &IconVariantService{basePath: filepath.Join(tempDir, "icons") + "/"}

	programName := "test-game-delete"
	itemName := "Health Potion"
	iconsPath := filepath.Join(tempDir, "icons", programName)

	t.Run("Delete existing variant", func(t *testing.T) {
		// Create variant file
		variantPath := filepath.Join(iconsPath, "Health Potion|Ice.png")
		createTestIconFile(t, variantPath)

		// Verify file exists
		if _, err := os.Stat(variantPath); os.IsNotExist(err) {
			t.Fatal("Test file was not created")
		}

		// Delete variant
		err := service.DeleteVariant(programName, itemName, "Ice")
		if err != nil {
			t.Fatalf("Expected no error, got: %v", err)
		}

		// Verify file was deleted
		if _, err := os.Stat(variantPath); !os.IsNotExist(err) {
			t.Error("Variant file was not deleted")
		}
	})

	t.Run("Delete non-existent variant (idempotent)", func(t *testing.T) {
		// Delete should not error even if file doesn't exist
		err := service.DeleteVariant(programName, itemName, "NonExistent")
		if err != nil {
			t.Errorf("Delete should be idempotent, got error: %v", err)
		}
	})

	t.Run("Delete legacy variant", func(t *testing.T) {
		// Create legacy icon
		legacyPath := filepath.Join(iconsPath, "Mana Potion.png")
		createTestIconFile(t, legacyPath)

		// Delete with empty variant name
		err := service.DeleteVariant(programName, "Mana Potion", "")
		if err != nil {
			t.Fatalf("Expected no error, got: %v", err)
		}

		// Verify file was deleted
		if _, err := os.Stat(legacyPath); !os.IsNotExist(err) {
			t.Error("Legacy icon was not deleted")
		}
	})

	t.Run("Empty parameters", func(t *testing.T) {
		tests := []struct {
			name        string
			programName string
			itemName    string
		}{
			{"Empty program name", "", itemName},
			{"Empty item name", programName, ""},
		}

		for _, tt := range tests {
			t.Run(tt.name, func(t *testing.T) {
				err := service.DeleteVariant(tt.programName, tt.itemName, "Test")
				if err == nil {
					t.Error("Expected error for empty parameter")
				}
			})
		}
	})
}
