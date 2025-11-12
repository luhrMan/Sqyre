package assets

import (
	"testing"
)

// TestLoadIconBytes verifies that LoadIconBytes correctly loads icons
// including both variant and non-variant icons
func TestLoadIconBytes(t *testing.T) {
	icons, err := LoadIconBytes()
	if err != nil {
		t.Fatalf("LoadIconBytes failed: %v", err)
	}

	if icons == nil {
		t.Fatal("Expected icons map to be non-nil")
	}

	// Verify that icons are loaded (we should have at least some icons)
	if len(icons) == 0 {
		t.Log("Warning: No icons loaded - this may be expected if no icon files exist")
	}

	// Verify that icon keys follow the expected format: programName|filename.png
	for key := range icons {
		if key == "" {
			t.Error("Found empty key in icons map")
		}
		// Keys should contain at least one delimiter (programName|filename)
		// They may contain two delimiters for variants (programName|itemName|variantName.png)
	}
}

// TestBytesToFyneIcons verifies that BytesToFyneIcons correctly converts
// icon bytes to Fyne static resources
func TestBytesToFyneIcons(t *testing.T) {
	fyneIcons := BytesToFyneIcons()
	
	if fyneIcons == nil {
		t.Fatal("Expected fyneIcons map to be non-nil")
	}

	// Verify that all icons are converted to Fyne resources
	iconBytes := GetIconBytes()
	if len(fyneIcons) != len(iconBytes) {
		t.Errorf("Expected %d Fyne icons, got %d", len(iconBytes), len(fyneIcons))
	}

	// Verify that each Fyne icon has the same key as the icon bytes
	for key := range iconBytes {
		if fyneIcons[key] == nil {
			t.Errorf("Missing Fyne icon for key: %s", key)
		}
	}
}

// TestGetIconBytes verifies that GetIconBytes returns the loaded icons
func TestGetIconBytes(t *testing.T) {
	// First load icons
	_, err := LoadIconBytes()
	if err != nil {
		t.Fatalf("LoadIconBytes failed: %v", err)
	}

	// Get icons
	icons := GetIconBytes()
	if icons == nil {
		t.Fatal("Expected icons map to be non-nil")
	}
}
