package services

import (
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

		// Add first variant
		err := service.AddVariant(programName, itemName, "Red", sourcePNG1)
		if err != nil {
			t.Fatalf("Failed to add Red variant: %v", err)
		}

		// Add second variant
		err = service.AddVariant(programName, itemName, "Blue", sourcePNG2)
		if err != nil {
			t.Fatalf("Failed to add Blue variant: %v", err)
		}

		// Verify files exist in filesystem
		variant1Path := service.GetVariantPath(programName, itemName, "Red")
		variant2Path := service.GetVariantPath(programName, itemName, "Blue")
		
		if _, err := os.Stat(variant1Path); os.IsNotExist(err) {
			t.Error("Red variant file should exist")
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
		expectedVariants := []string{"Blue", "Red"}
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

		// Add first variant
		err := service.AddVariant(programName, itemName, "Original", sourcePNG1)
		if err != nil {
			t.Fatalf("Failed to add Original variant: %v", err)
		}

		// Try to add duplicate variant name
		err = service.AddVariant(programName, itemName, "Original", sourcePNG2)
		if err == nil {
			t.Error("Should not allow duplicate variant names")
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

		// Try various path traversal attempts
		badNames := []string{
			"../../../etc/passwd",
			"..\\..\\windows\\system32",
			"../../badpath",
			"subdir/variant",
		}

		for _, badName := range badNames {
			err := service.AddVariant(programName, itemName, badName, sourcePNG1)
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

	// Add two variants
	err = service.AddVariant(programName, itemName, "Variant1", sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add Variant1: %v", err)
	}
	err = service.AddVariant(programName, itemName, "Variant2", sourcePNG)
	if err != nil {
		t.Fatalf("Failed to add Variant2: %v", err)
	}

	// Verify both files exist
	variant1Path := service.GetVariantPath(programName, itemName, "Variant1")
	variant2Path := service.GetVariantPath(programName, itemName, "Variant2")
	
	if _, err := os.Stat(variant1Path); os.IsNotExist(err) {
		t.Fatal("Variant 1 should exist before deletion")
	}
	if _, err := os.Stat(variant2Path); os.IsNotExist(err) {
		t.Fatal("Variant 2 should exist before deletion")
	}

	// Delete variant 1
	err = service.DeleteVariant(programName, itemName, "Variant1")
	if err != nil {
		t.Fatalf("Failed to delete variant 1: %v", err)
	}

	// Verify variant 1 is deleted
	if _, err := os.Stat(variant1Path); !os.IsNotExist(err) {
		t.Error("Variant 1 should be deleted")
	}

	// Verify variant 2 still exists
	if _, err := os.Stat(variant2Path); os.IsNotExist(err) {
		t.Error("Variant 2 should still exist")
	}

	// Verify GetVariants returns only remaining variant
	variants, err := service.GetVariants(programName, itemName)
	if err != nil {
		t.Fatalf("Failed to get variants: %v", err)
	}
	if len(variants) != 1 {
		t.Errorf("Expected 1 variant after deletion, got %d", len(variants))
	}
	if len(variants) > 0 && variants[0] != "Variant2" {
		t.Errorf("Expected remaining variant to be 'Variant2', got '%s'", variants[0])
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
	programs := []struct {
		name     string
		itemName string
		variants []string
	}{
		{"dark and darker", "Health Potion", []string{"Red", "Blue"}},
		{"path of exile 2", "Chaos Orb", []string{"Normal", "Shiny"}},
		{"another game", "Gold Coin", []string{"Small", "Large", "Huge"}},
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

		if len(discoveredVariants) != len(prog.variants) {
			t.Errorf("Expected %d variants for %s, found %d", len(prog.variants), prog.name, len(discoveredVariants))
		}

		// Verify variant files exist
		for _, variant := range prog.variants {
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
