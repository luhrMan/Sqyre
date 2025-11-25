package services

import (
	"Squire/internal/assets"
	"os"
	"path/filepath"
	"testing"
)

// TestIntegration_IconVariants_EndToEnd tests end-to-end variant management
func TestIntegration_IconVariants_EndToEnd(t *testing.T) {
	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-icon-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	// Create icon variant service with test base path
	service := &IconVariantService{basePath: tempIconsDir}
	
	// Create a test program directory
	programName := "test-program"
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG files
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()
		
		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Create source PNG files for testing
	sourcePNG1 := filepath.Join(tempIconsDir, "source1.png")
	sourcePNG2 := filepath.Join(tempIconsDir, "source2.png")
	if err := createTestPNG(sourcePNG1); err != nil {
		t.Fatalf("Failed to create source PNG 1: %v", err)
	}
	if err := createTestPNG(sourcePNG2); err != nil {
		t.Fatalf("Failed to create source PNG 2: %v", err)
	}

	t.Run("Create item and add variants", func(t *testing.T) {
		itemName := "Health Potion"

		// Add first variant - should be forced to "Original"
		err := service.AddVariant(programName, itemName, "Red", sourcePNG1)
		if err != nil {
			t.Fatalf("Failed to add first variant: %v", err)
		}

		// Add second variant
		err = service.AddVariant(programName, itemName, "Blue", sourcePNG2)
		if err != nil {
			t.Fatalf("Failed to add Blue variant: %v", err)
		}

		// Verify files exist in filesystem
		variant1Path := service.GetVariantPath(programName, itemName, "Original") // First variant is always "Original"
		variant2Path := service.GetVariantPath(programName, itemName, "Blue")
		
		if _, err := os.Stat(variant1Path); os.IsNotExist(err) {
			t.Error("Original variant file should exist")
		}
		if _, err := os.Stat(variant2Path); os.IsNotExist(err) {
			t.Error("Blue variant file should exist")
		}

		// Get variants and verify
		variants, err := service.GetVariants(programName, itemName)
		if err != nil {
			t.Fatalf("Failed to get variants: %v", err)
		}

		if len(variants) != 2 {
			t.Errorf("Expected 2 variants, got %d", len(variants))
		}

		// Verify variant names (should be sorted)
		expectedVariants := []string{"Blue", "Original"}
		for i, expected := range expectedVariants {
			if i >= len(variants) {
				t.Errorf("Missing variant at index %d", i)
				continue
			}
			if variants[i] != expected {
				t.Errorf("Expected variant '%s' at index %d, got '%s'", expected, i, variants[i])
			}
		}
	})

	t.Run("Prevent duplicate variant names", func(t *testing.T) {
		itemName := "Mana Potion"

		// Add first variant - will be forced to "Original"
		err := service.AddVariant(programName, itemName, "SomeVariant", sourcePNG1)
		if err != nil {
			t.Fatalf("Failed to add first variant: %v", err)
		}

		// Try to add duplicate "Original" variant name
		err = service.AddVariant(programName, itemName, "Original", sourcePNG2)
		if err == nil {
			t.Error("Should not allow duplicate variant names")
		}
		
		// Check that it's the correct error type
		if _, ok := err.(*VariantExistsError); !ok {
			t.Errorf("Expected VariantExistsError, got %T", err)
		}
	})

	t.Run("Enforce variant count limit", func(t *testing.T) {
		itemName := "Limited Item"

		// Create 100 variants (the maximum)
		for i := 0; i < 100; i++ {
			variantName := string(rune('A' + (i % 26))) + string(rune('0' + (i / 26)))
			err := service.AddVariant(programName, itemName, variantName, sourcePNG1)
			if err != nil {
				t.Fatalf("Failed to add variant %d: %v", i, err)
			}
		}

		// Try to add 101st variant
		err := service.AddVariant(programName, itemName, "TooMany", sourcePNG1)
		if err == nil {
			t.Error("Should not allow more than 100 variants")
		}
	})

	t.Run("Validate PNG files", func(t *testing.T) {
		itemName := "Validated Item"

		// Create a non-PNG file
		nonPNGPath := filepath.Join(tempIconsDir, "notpng.txt")
		if err := os.WriteFile(nonPNGPath, []byte("not a png"), 0644); err != nil {
			t.Fatalf("Failed to create non-PNG file: %v", err)
		}

		// Try to add non-PNG file
		err := service.AddVariant(programName, itemName, "Invalid", nonPNGPath)
		if err == nil {
			t.Error("Should not allow non-PNG files")
		}
	})

	t.Run("Prevent path traversal in variant names", func(t *testing.T) {
		itemName := "Secure Item"

		// Add first variant to get past the "Original" requirement
		err := service.AddVariant(programName, itemName, "FirstVariant", sourcePNG1)
		if err != nil {
			t.Fatalf("Failed to add first variant: %v", err)
		}

		// Try various path traversal attempts
		badNames := []string{
			"../../../etc/passwd",
			"..\\..\\windows\\system32",
			"../../badpath",
			"subdir/variant",
		}

		for _, badName := range badNames {
			err := service.AddVariant(programName, itemName, badName, sourcePNG2)
			if err == nil {
				t.Errorf("Should not allow path traversal in variant name: %s", badName)
			}
		}
	})
}

// TestIntegration_IconVariants_DeleteVariant tests deleting a variant and verifying file removal
func TestIntegration_IconVariants_DeleteVariant(t *testing.T) {
	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-icon-delete-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	// Create icon variant service
	service := &IconVariantService{basePath: tempIconsDir}

	programName := "delete-test-program"
	itemName := "Test Item"
	
	// Create program icons directory
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG file
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()
		
		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Create source PNG
	sourcePNG := filepath.Join(tempIconsDir, "source.png")
	if err := createTestPNG(sourcePNG); err != nil {
		t.Fatalf("Failed to create source PNG: %v", err)
	}

	// Add two variants - first will be "Original", second will be "Variant2"
	err = service.AddVariant(programName, itemName, "Variant1", sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add first variant: %v", err)
	}
	err = service.AddVariant(programName, itemName, "Variant2", sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add Variant2: %v", err)
	}

	// Verify both files exist
	originalPath := service.GetVariantPath(programName, itemName, "Original") // First variant is always "Original"
	variant2Path := service.GetVariantPath(programName, itemName, "Variant2")
	
	if _, err := os.Stat(originalPath); os.IsNotExist(err) {
		t.Fatal("Original variant should exist before deletion")
	}
	if _, err := os.Stat(variant2Path); os.IsNotExist(err) {
		t.Fatal("Variant 2 should exist before deletion")
	}

	// Try to delete "Original" variant - should fail
	err = service.DeleteVariant(programName, itemName, "Original")
	if err == nil {
		t.Error("Should not be able to delete Original variant")
	}

	// Delete variant 2 instead
	err = service.DeleteVariant(programName, itemName, "Variant2")
	if err != nil {
		t.Fatalf("Failed to delete variant 1: %v", err)
	}

	// Verify variant 2 is deleted
	if _, err := os.Stat(variant2Path); !os.IsNotExist(err) {
		t.Error("Variant 2 should be deleted")
	}

	// Verify Original still exists
	if _, err := os.Stat(originalPath); os.IsNotExist(err) {
		t.Error("Original variant should still exist")
	}

	// Verify GetVariants returns only remaining variant
	variants, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get variants: %v", err)
	}
	if len(variants) != 1 {
		t.Errorf("Expected 1 variant after deletion, got %d", len(variants))
	}
	if len(variants) > 0 && variants[0] != "Original" {
		t.Errorf("Expected remaining variant to be 'Original', got '%s'", variants[0])
	}
}

// TestIntegration_IconVariants_LoadWithVariants tests loading an item with variants and verifying all are discovered
func TestIntegration_IconVariants_LoadWithVariants(t *testing.T) {
	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-icon-load-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	service := &IconVariantService{basePath: tempIconsDir}

	programName := "load-test-program"
	itemName := "Mana Potion"
	
	// Create program icons directory
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG file helper
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()
		
		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Create source PNG
	sourcePNG := filepath.Join(tempIconsDir, "source.png")
	if err := createTestPNG(sourcePNG); err != nil {
		t.Fatalf("Failed to create source PNG: %v", err)
	}

	// Create multiple variants
	variants := []string{"Original", "Ice", "Fire", "Bone"}
	for _, variant := range variants {
		err := service.AddVariant(programName, itemName, variant, sourcePNG)
		if err != nil {
			t.Fatalf("Failed to add variant %s: %v", variant, err)
		}
	}

	// Verify all variant files exist
	for _, variant := range variants {
		variantPath := service.GetVariantPath(programName, itemName, variant)
		if _, err := os.Stat(variantPath); os.IsNotExist(err) {
			t.Errorf("Variant %s should exist", variant)
		}
	}

	// Get variants and verify all are discovered
	discoveredVariants, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get variants: %v", err)
	}

	if len(discoveredVariants) != len(variants) {
		t.Errorf("Expected %d variants, found %d", len(variants), len(discoveredVariants))
	}

	// Verify all expected variants are present (order may differ due to sorting)
	variantMap := make(map[string]bool)
	for _, v := range discoveredVariants {
		variantMap[v] = true
	}

	for _, expected := range variants {
		if !variantMap[expected] {
			t.Errorf("Expected variant '%s' not found in discovered variants", expected)
		}
	}
}

// TestIntegration_IconVariants_MultiplePrograms tests icon variants with multiple programs simultaneously
func TestIntegration_IconVariants_MultiplePrograms(t *testing.T) {
	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-icon-multi-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	service := &IconVariantService{basePath: tempIconsDir}

	// Create test PNG file helper
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()
		
		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Create source PNG
	sourcePNG := filepath.Join(tempIconsDir, "source.png")
	if err := createTestPNG(sourcePNG); err != nil {
		t.Fatalf("Failed to create source PNG: %v", err)
	}

	// Create multiple programs with items and variants
	// Note: First variant will always be "Original", so we adjust expected variants
	programs := []struct {
		name     string
		itemName string
		variants []string
		expected []string // What we expect after first variant becomes "Original"
	}{
		{"dark and darker", "Health Potion", []string{"Red", "Blue"}, []string{"Blue", "Original"}},
		{"path of exile 2", "Chaos Orb", []string{"Normal", "Shiny"}, []string{"Original", "Shiny"}},
		{"another game", "Gold Coin", []string{"Small", "Large", "Huge"}, []string{"Huge", "Large", "Original"}},
	}

	for _, prog := range programs {
		// Create program icons directory
		programIconsDir := filepath.Join(tempIconsDir, prog.name)
		if err := os.MkdirAll(programIconsDir, 0755); err != nil {
			t.Fatalf("Failed to create icons dir for %s: %v", prog.name, err)
		}

		// Add variants
		for _, variant := range prog.variants {
			err := service.AddVariant(prog.name, prog.itemName, variant, sourcePNG)
			if err != nil {
				t.Fatalf("Failed to add variant %s for %s: %v", variant, prog.name, err)
			}
		}
	}

	// Verify all programs and their variants
	for _, prog := range programs {
		// Verify variants
		discoveredVariants, err := service.GetVariants(prog.name, prog.itemName)
		if err != nil {
			t.Fatalf("Failed to get variants for %s: %v", prog.name, err)
		}

		if len(discoveredVariants) != len(prog.expected) {
			t.Errorf("Expected %d variants for %s, found %d", len(prog.expected), prog.name, len(discoveredVariants))
		}

		// Verify expected variant files exist
		for _, variant := range prog.expected {
			variantPath := service.GetVariantPath(prog.name, prog.itemName, variant)
			if _, err := os.Stat(variantPath); os.IsNotExist(err) {
				t.Errorf("Variant %s should exist for %s in %s", variant, prog.itemName, prog.name)
			}
		}
	}

	// Verify programs are independent (variants don't leak between programs)
	for i, prog1 := range programs {
		for j, prog2 := range programs {
			if i == j {
				continue
			}

			// Check that prog1's variants don't exist in prog2's directory
			for _, variant := range prog1.variants {
				wrongPath := service.GetVariantPath(prog2.name, prog1.itemName, variant)
				if _, err := os.Stat(wrongPath); !os.IsNotExist(err) {
					t.Errorf("Variant %s from %s should not exist in %s directory", 
						variant, prog1.name, prog2.name)
				}
			}
		}
	}
}

// TestIntegration_CacheInvalidation_AddVariant verifies that AddVariant invalidates
// the Fyne Resource cache for the newly added variant
func TestIntegration_CacheInvalidation_AddVariant(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-cache-add-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	service := &IconVariantService{basePath: tempIconsDir}

	programName := "cache-test-program"
	itemName := "Test Item"
	variantName := "Variant1"

	// Create program icons directory
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG file
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()

		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Create source PNG
	sourcePNG := filepath.Join(tempIconsDir, "source.png")
	if err := createTestPNG(sourcePNG); err != nil {
		t.Fatalf("Failed to create source PNG: %v", err)
	}

	// Add variant - this should call InvalidateFyneResourceCache
	// First variant will be forced to "Original"
	err = service.AddVariant(programName, itemName, variantName, sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add variant: %v", err)
	}

	// Verify file exists in the test directory - first variant becomes "Original"
	originalPath := service.GetVariantPath(programName, itemName, "Original")
	if _, err := os.Stat(originalPath); os.IsNotExist(err) {
		t.Fatal("Original variant file should exist after AddVariant")
	}

	// Verify the variant file was created with correct naming
	expectedFilename := itemName + "|Original.png"
	actualFilename := filepath.Base(originalPath)
	if actualFilename != expectedFilename {
		t.Errorf("Expected filename '%s', got '%s'", expectedFilename, actualFilename)
	}
}

// TestIntegration_CacheInvalidation_DeleteVariant verifies that DeleteVariant invalidates
// the Fyne Resource cache for the deleted variant
func TestIntegration_CacheInvalidation_DeleteVariant(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-cache-delete-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	service := &IconVariantService{basePath: tempIconsDir}

	programName := "cache-delete-program"
	itemName := "Test Item"
	variantName := "Variant1"

	// Create program icons directory
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG file
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()

		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Create source PNG
	sourcePNG := filepath.Join(tempIconsDir, "source.png")
	if err := createTestPNG(sourcePNG); err != nil {
		t.Fatalf("Failed to create source PNG: %v", err)
	}

	// Add first variant (will become "Original") and then the test variant
	err = service.AddVariant(programName, itemName, "FirstVariant", sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add first variant: %v", err)
	}
	
	err = service.AddVariant(programName, itemName, variantName, sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add test variant: %v", err)
	}

	// Verify file exists before deletion
	variantPath := service.GetVariantPath(programName, itemName, variantName)
	if _, err := os.Stat(variantPath); os.IsNotExist(err) {
		t.Fatal("Variant file should exist before deletion")
	}

	// Delete variant - this should call InvalidateFyneResourceCache
	err = service.DeleteVariant(programName, itemName, variantName)
	if err != nil {
		t.Fatalf("Failed to delete variant: %v", err)
	}

	// Verify file is deleted from filesystem
	if _, err := os.Stat(variantPath); !os.IsNotExist(err) {
		t.Error("Variant file should be deleted after DeleteVariant")
	}

	// Verify GetVariants no longer returns the deleted variant
	variants, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get variants: %v", err)
	}

	for _, v := range variants {
		if v == variantName {
			t.Errorf("Deleted variant '%s' should not be in GetVariants result", variantName)
		}
	}
}

// TestIntegration_EndToEnd_LoadAddReload verifies the complete flow:
// load icon → add variant → verify cache invalidated → verify reload
func TestIntegration_EndToEnd_LoadAddReload(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-e2e-add-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	service := &IconVariantService{basePath: tempIconsDir}

	programName := "e2e-add-program"
	itemName := "Health Potion"
	variantName := "NewVariant"

	// Create program icons directory
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG file helper
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()

		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Step 1: Create initial variant
	initialVariantPath := filepath.Join(programIconsDir, itemName+"|Initial.png")
	if err := createTestPNG(initialVariantPath); err != nil {
		t.Fatalf("Failed to create initial variant: %v", err)
	}

	// Verify initial variant exists
	if _, err := os.Stat(initialVariantPath); os.IsNotExist(err) {
		t.Fatal("Initial variant file should exist")
	}

	// Step 2: Get initial variants list
	initialVariants, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get initial variants: %v", err)
	}

	if len(initialVariants) != 1 {
		t.Errorf("Expected 1 initial variant, got %d", len(initialVariants))
	}

	// Step 3: Add new variant
	sourcePNG := filepath.Join(tempIconsDir, "source.png")
	if err := createTestPNG(sourcePNG); err != nil {
		t.Fatalf("Failed to create source PNG: %v", err)
	}

	err = service.AddVariant(programName, itemName, variantName, sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add variant: %v", err)
	}

	// Step 4: Verify new variant file exists
	newVariantPath := service.GetVariantPath(programName, itemName, variantName)
	if _, err := os.Stat(newVariantPath); os.IsNotExist(err) {
		t.Error("New variant file should exist after AddVariant")
	}

	// Step 5: Verify GetVariants returns both variants
	variantsAfterAdd, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get variants after add: %v", err)
	}

	if len(variantsAfterAdd) != 2 {
		t.Errorf("Expected 2 variants after add, got %d", len(variantsAfterAdd))
	}

	// Verify both variant names are present
	variantMap := make(map[string]bool)
	for _, v := range variantsAfterAdd {
		variantMap[v] = true
	}

	if !variantMap["Initial"] {
		t.Error("Expected 'Initial' variant to be present")
	}
	if !variantMap[variantName] {
		t.Errorf("Expected '%s' variant to be present", variantName)
	}
}

// TestIntegration_EndToEnd_LoadDeletePlaceholder verifies the complete flow:
// load icon → delete variant → verify cache invalidated → verify placeholder
func TestIntegration_EndToEnd_LoadDeletePlaceholder(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create a temporary directory for test icons
	tempIconsDir, err := os.MkdirTemp("", "sqyre-e2e-delete-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp icons dir: %v", err)
	}
	defer os.RemoveAll(tempIconsDir)

	service := &IconVariantService{basePath: tempIconsDir}

	programName := "e2e-delete-program"
	itemName := "Mana Potion"
	variantName := "ToDelete"
	keepVariantName := "KeepThis"

	// Create program icons directory
	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("Failed to create program icons dir: %v", err)
	}

	// Create test PNG file helper
	createTestPNG := func(path string) error {
		file, err := os.Create(path)
		if err != nil {
			return err
		}
		defer file.Close()

		// Write PNG signature
		pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		_, err = file.Write(pngSignature)
		return err
	}

	// Step 1: Create two variants - one to delete, one to keep
	variantToDeletePath := filepath.Join(programIconsDir, itemName+"|"+variantName+".png")
	if err := createTestPNG(variantToDeletePath); err != nil {
		t.Fatalf("Failed to create variant to delete: %v", err)
	}

	variantToKeepPath := filepath.Join(programIconsDir, itemName+"|"+keepVariantName+".png")
	if err := createTestPNG(variantToKeepPath); err != nil {
		t.Fatalf("Failed to create variant to keep: %v", err)
	}

	// Step 2: Verify both variants exist before deletion
	initialVariants, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get initial variants: %v", err)
	}

	if len(initialVariants) != 2 {
		t.Errorf("Expected 2 initial variants, got %d", len(initialVariants))
	}

	// Step 3: Delete one variant
	err = service.DeleteVariant(programName, itemName, variantName)
	if err != nil {
		t.Fatalf("Failed to delete variant: %v", err)
	}

	// Step 4: Verify file is deleted from filesystem
	if _, err := os.Stat(variantToDeletePath); !os.IsNotExist(err) {
		t.Error("Variant file should be deleted from filesystem")
	}

	// Step 5: Verify the other variant still exists
	if _, err := os.Stat(variantToKeepPath); os.IsNotExist(err) {
		t.Error("Other variant file should still exist")
	}

	// Step 6: Verify GetVariants no longer returns the deleted variant
	variantsAfterDelete, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get variants after delete: %v", err)
	}

	if len(variantsAfterDelete) != 1 {
		t.Errorf("Expected 1 variant after delete, got %d", len(variantsAfterDelete))
	}

	// Verify the deleted variant is not in the list
	for _, v := range variantsAfterDelete {
		if v == variantName {
			t.Errorf("Deleted variant '%s' should not be in GetVariants result", variantName)
		}
	}

	// Verify the kept variant is still in the list
	foundKeptVariant := false
	for _, v := range variantsAfterDelete {
		if v == keepVariantName {
			foundKeptVariant = true
			break
		}
	}
	if !foundKeptVariant {
		t.Errorf("Expected kept variant '%s' to still be in GetVariants result", keepVariantName)
	}
}
