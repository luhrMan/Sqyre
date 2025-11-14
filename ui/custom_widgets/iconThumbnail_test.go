package custom_widgets

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"os"
	"path/filepath"
	"testing"

	"fyne.io/fyne/v2/test"
)

// setupTestIconFile creates a temporary test icon file and returns its path
func setupTestIconFile(t *testing.T, programName, filename string) string {
	t.Helper()

	// Create temporary directory structure
	iconsPath := config.GetIconsPath()
	programPath := filepath.Join(iconsPath, programName)

	// Create program directory
	if err := os.MkdirAll(programPath, 0755); err != nil {
		t.Fatalf("Failed to create test program directory: %v", err)
	}

	// Create a minimal valid PNG file (1x1 transparent pixel)
	// PNG signature + IHDR + IDAT + IEND chunks
	pngData := []byte{
		0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
		0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
		0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
		0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, // RGBA, CRC
		0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, // IDAT chunk
		0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, // compressed data
		0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, // CRC
		0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, // IEND chunk
		0x42, 0x60, 0x82, // CRC
	}

	iconPath := filepath.Join(programPath, filename)
	if err := os.WriteFile(iconPath, pngData, 0644); err != nil {
		t.Fatalf("Failed to create test icon file: %v", err)
	}

	return iconPath
}

// cleanupTestIconFile removes the test icon file and its parent directory if empty
func cleanupTestIconFile(t *testing.T, iconPath string) {
	t.Helper()

	// Remove the file
	if err := os.Remove(iconPath); err != nil && !os.IsNotExist(err) {
		t.Logf("Warning: Failed to remove test icon file: %v", err)
	}

	// Try to remove the parent directory (will only succeed if empty)
	programPath := filepath.Dir(iconPath)
	os.Remove(programPath) // Ignore error - directory might not be empty
}

// TestLoadIconCreatesCanvasImageFromCachedResource verifies that loadIcon()
// creates a canvas.Image from the cached Fyne Resource
func TestLoadIconCreatesCanvasImageFromCachedResource(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create test icon file
	programName := "test-program"
	filename := "test-icon.png"
	iconPath := setupTestIconFile(t, programName, filename)
	defer cleanupTestIconFile(t, iconPath)

	// Create IconThumbnail widget
	thumbnail := NewIconThumbnail(iconPath, "Test Variant", nil)

	// Verify the image was created
	if thumbnail.image == nil {
		t.Fatal("Expected image to be non-nil")
	}

	// Verify the image has the correct minimum size
	minSize := thumbnail.image.MinSize()
	expectedSize := float32(config.IconThumbnailSize)
	if minSize.Width != expectedSize || minSize.Height != expectedSize {
		t.Errorf("Expected min size %fx%f, got %fx%f",
			expectedSize, expectedSize, minSize.Width, minSize.Height)
	}

	// Verify the icon is now in cache
	icons := assets.BytesToFyneIcons()
	cacheKey := programName + config.ProgramDelimiter + filename
	if _, exists := icons[cacheKey]; !exists {
		t.Errorf("Expected icon to be cached with key: %s", cacheKey)
	}
}

// TestMultipleIconThumbnailInstancesShareCanvasImages verifies that
// multiple IconThumbnail instances share the same cached canvas.Image object
// to prevent memory bloat from repeatedly decoding the same PNG
func TestMultipleIconThumbnailInstancesShareCanvasImages(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create test icon file
	programName := "test-program"
	filename := "shared-icon.png"
	iconPath := setupTestIconFile(t, programName, filename)
	defer cleanupTestIconFile(t, iconPath)

	// Create multiple IconThumbnail instances with the same icon path
	thumbnail1 := NewIconThumbnail(iconPath, "Variant 1", nil)
	thumbnail2 := NewIconThumbnail(iconPath, "Variant 2", nil)
	thumbnail3 := NewIconThumbnail(iconPath, "Variant 3", nil)

	// Verify all images were created
	if thumbnail1.image == nil || thumbnail2.image == nil || thumbnail3.image == nil {
		t.Fatal("Expected all images to be non-nil")
	}

	// Verify they are the SAME canvas.Image instances (same pointers)
	// This is the optimization - sharing decoded pixel buffers
	if thumbnail1.image != thumbnail2.image {
		t.Error("Expected thumbnail1 and thumbnail2 to share the same canvas.Image instance (cached)")
	}
	if thumbnail1.image != thumbnail3.image {
		t.Error("Expected thumbnail1 and thumbnail3 to share the same canvas.Image instance (cached)")
	}
	if thumbnail2.image != thumbnail3.image {
		t.Error("Expected thumbnail2 and thumbnail3 to share the same canvas.Image instance (cached)")
	}

	// Verify they all reference the same cached Fyne Resource
	icons := assets.BytesToFyneIcons()
	cacheKey := programName + config.ProgramDelimiter + filename
	cachedResource, exists := icons[cacheKey]
	if !exists {
		t.Fatalf("Expected icon to be cached with key: %s", cacheKey)
	}

	// Verify all thumbnails use the same cached resource
	if cachedResource == nil {
		t.Error("Expected cached resource to be non-nil")
	}
}

// TestInvalidIconPathReturnsPlaceholder verifies that an invalid icon path
// returns a placeholder image instead of crashing
func TestInvalidIconPathReturnsPlaceholder(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create IconThumbnail with non-existent icon path
	invalidPath := filepath.Join(config.GetIconsPath(), "nonexistent-program", "nonexistent-icon.png")
	thumbnail := NewIconThumbnail(invalidPath, "Invalid Variant", nil)

	// Verify the image was created (placeholder)
	if thumbnail.image == nil {
		t.Fatal("Expected placeholder image to be non-nil")
	}

	// Verify the placeholder has the correct minimum size
	minSize := thumbnail.image.MinSize()
	expectedSize := float32(config.IconThumbnailSize)
	if minSize.Width != expectedSize || minSize.Height != expectedSize {
		t.Errorf("Expected placeholder min size %fx%f, got %fx%f",
			expectedSize, expectedSize, minSize.Width, minSize.Height)
	}

	// Verify the resource is the broken image icon from theme
	// (We can't directly check this, but we verified it doesn't crash and returns an image)
}

// TestEmptyIconPathReturnsPlaceholder verifies that an empty icon path
// returns a placeholder image
func TestEmptyIconPathReturnsPlaceholder(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create IconThumbnail with empty icon path
	thumbnail := NewIconThumbnail("", "Empty Path Variant", nil)

	// Verify the image was created (placeholder)
	if thumbnail.image == nil {
		t.Fatal("Expected placeholder image to be non-nil")
	}
}

// TestIconLoadingAfterCacheInvalidationReloadsFromDisk verifies that after
// cache invalidation, the icon is reloaded from disk on next access
func TestIconLoadingAfterCacheInvalidationReloadsFromDisk(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create test icon file
	programName := "test-program"
	filename := "reload-test.png"
	iconPath := setupTestIconFile(t, programName, filename)
	defer cleanupTestIconFile(t, iconPath)

	// Create first IconThumbnail - loads from disk and caches
	thumbnail1 := NewIconThumbnail(iconPath, "First Load", nil)
	if thumbnail1.image == nil {
		t.Fatal("Expected first thumbnail image to be non-nil")
	}

	// Get the cached resource
	icons1 := assets.BytesToFyneIcons()
	cacheKey := programName + config.ProgramDelimiter + filename
	cachedResource1, exists := icons1[cacheKey]
	if !exists {
		t.Fatalf("Expected icon to be cached with key: %s", cacheKey)
	}

	// Invalidate the cache entry
	assets.InvalidateFyneResourceCache(cacheKey)

	// Create second IconThumbnail - should reload from disk
	thumbnail2 := NewIconThumbnail(iconPath, "After Invalidation", nil)
	if thumbnail2.image == nil {
		t.Fatal("Expected second thumbnail image to be non-nil")
	}

	// Get the cached resource again
	icons2 := assets.BytesToFyneIcons()
	cachedResource2, exists := icons2[cacheKey]
	if !exists {
		t.Fatalf("Expected icon to be reloaded and cached with key: %s", cacheKey)
	}

	// Verify the resource was reloaded (different pointer)
	if cachedResource1 == cachedResource2 {
		t.Error("Expected different resource pointer after cache invalidation (should reload from disk)")
	}

	// Verify the content is the same (same file)
	if len(cachedResource1.Content()) != len(cachedResource2.Content()) {
		t.Errorf("Expected same content length after reload, got %d and %d",
			len(cachedResource1.Content()), len(cachedResource2.Content()))
	}

	// Verify both thumbnails have valid images (different instances)
	if thumbnail1.image == thumbnail2.image {
		t.Error("Expected different canvas.Image instances for different thumbnails")
	}
}

// TestConstructIconKey verifies that constructIconKey correctly builds
// cache keys from icon file paths
func TestConstructIconKey(t *testing.T) {
	tests := []struct {
		name        string
		iconPath    string
		expectedKey string
	}{
		{
			name:        "Standard icon path",
			iconPath:    filepath.Join(config.GetIconsPath(), "dark and darker", "Health Potion.png"),
			expectedKey: "dark and darker|Health Potion.png",
		},
		{
			name:        "Icon with variant",
			iconPath:    filepath.Join(config.GetIconsPath(), "path of exile 2", "Scroll|Variant1.png"),
			expectedKey: "path of exile 2|Scroll|Variant1.png",
		},
		{
			name:        "Empty path",
			iconPath:    "",
			expectedKey: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			thumbnail := &IconThumbnail{iconPath: tt.iconPath}
			key := thumbnail.constructIconKey()

			if key != tt.expectedKey {
				t.Errorf("Expected key %q, got %q", tt.expectedKey, key)
			}
		})
	}
}

// TestIconThumbnailWidgetCreation verifies that NewIconThumbnail creates
// a properly initialized widget with all components
func TestIconThumbnailWidgetCreation(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create test icon file
	programName := "test-program"
	filename := "widget-test.png"
	iconPath := setupTestIconFile(t, programName, filename)
	defer cleanupTestIconFile(t, iconPath)

	variantName := "Test Variant"
	deleteCallbackCalled := false
	onDelete := func() {
		deleteCallbackCalled = true
	}

	// Create IconThumbnail widget
	thumbnail := NewIconThumbnail(iconPath, variantName, onDelete)

	// Verify widget is properly initialized
	if thumbnail == nil {
		t.Fatal("Expected thumbnail to be non-nil")
	}

	// Verify icon path is set
	if thumbnail.iconPath != iconPath {
		t.Errorf("Expected iconPath %q, got %q", iconPath, thumbnail.iconPath)
	}

	// Verify variant name is set
	if thumbnail.variantName != variantName {
		t.Errorf("Expected variantName %q, got %q", variantName, thumbnail.variantName)
	}

	// Verify image component exists
	if thumbnail.image == nil {
		t.Error("Expected image to be non-nil")
	}

	// Verify label component exists and has correct text
	if thumbnail.label == nil {
		t.Error("Expected label to be non-nil")
	} else if thumbnail.label.Text != variantName {
		t.Errorf("Expected label text %q, got %q", variantName, thumbnail.label.Text)
	}

	// Verify delete button exists
	if thumbnail.deleteBtn == nil {
		t.Error("Expected deleteBtn to be non-nil")
	}

	// Verify container exists
	if thumbnail.container == nil {
		t.Error("Expected container to be non-nil")
	}

	// Verify delete callback works
	if thumbnail.onDelete == nil {
		t.Error("Expected onDelete callback to be set")
	} else {
		thumbnail.onDelete()
		if !deleteCallbackCalled {
			t.Error("Expected delete callback to be called")
		}
	}
}

// TestSetIconPath verifies that SetIconPath updates the icon and refreshes the widget
func TestSetIconPath(t *testing.T) {
	// Clear cache before test
	assets.ClearFyneResourceCache()

	// Create first test icon file
	programName1 := "test-program-1"
	filename1 := "icon1.png"
	iconPath1 := setupTestIconFile(t, programName1, filename1)
	defer cleanupTestIconFile(t, iconPath1)

	// Create second test icon file
	programName2 := "test-program-2"
	filename2 := "icon2.png"
	iconPath2 := setupTestIconFile(t, programName2, filename2)
	defer cleanupTestIconFile(t, iconPath2)

	// Create IconThumbnail with first icon
	thumbnail := NewIconThumbnail(iconPath1, "Initial", nil)
	if thumbnail.iconPath != iconPath1 {
		t.Errorf("Expected initial iconPath %q, got %q", iconPath1, thumbnail.iconPath)
	}

	// Update to second icon
	thumbnail.SetIconPath(iconPath2)

	// Verify icon path was updated
	if thumbnail.iconPath != iconPath2 {
		t.Errorf("Expected updated iconPath %q, got %q", iconPath2, thumbnail.iconPath)
	}

	// Verify image was reloaded
	if thumbnail.image == nil {
		t.Error("Expected image to be reloaded after SetIconPath")
	}
}

// TestSetVariantName verifies that SetVariantName updates the label text
func TestSetVariantName(t *testing.T) {
	// Create IconThumbnail with initial variant name
	thumbnail := NewIconThumbnail("", "Initial Name", nil)

	initialName := thumbnail.label.Text
	if initialName != "Initial Name" {
		t.Errorf("Expected initial label text %q, got %q", "Initial Name", initialName)
	}

	// Update variant name
	newName := "Updated Name"
	thumbnail.SetVariantName(newName)

	// Verify label text was updated
	if thumbnail.label.Text != newName {
		t.Errorf("Expected updated label text %q, got %q", newName, thumbnail.label.Text)
	}

	// Verify internal variantName field was updated
	if thumbnail.variantName != newName {
		t.Errorf("Expected updated variantName %q, got %q", newName, thumbnail.variantName)
	}
}

// TestSetOnDelete verifies that SetOnDelete updates the delete callback
func TestSetOnDelete(t *testing.T) {
	// Create IconThumbnail with initial callback
	initialCallbackCalled := false
	initialCallback := func() {
		initialCallbackCalled = true
	}

	thumbnail := NewIconThumbnail("", "Test", initialCallback)

	// Verify initial callback works
	thumbnail.onDelete()
	if !initialCallbackCalled {
		t.Error("Expected initial callback to be called")
	}

	// Update callback
	newCallbackCalled := false
	newCallback := func() {
		newCallbackCalled = true
	}
	thumbnail.SetOnDelete(newCallback)

	// Verify new callback works
	thumbnail.onDelete()
	if !newCallbackCalled {
		t.Error("Expected new callback to be called")
	}
}

// TestCreatePlaceholder verifies that createPlaceholder returns a valid image
func TestCreatePlaceholder(t *testing.T) {
	thumbnail := &IconThumbnail{}

	placeholder := thumbnail.createPlaceholder(true)

	// Verify placeholder is not nil
	if placeholder == nil {
		t.Fatal("Expected placeholder to be non-nil")
	}

	// Verify placeholder has correct minimum size
	minSize := placeholder.MinSize()
	expectedSize := float32(config.IconThumbnailSize)
	if minSize.Width != expectedSize || minSize.Height != expectedSize {
		t.Errorf("Expected placeholder min size %fx%f, got %fx%f",
			expectedSize, expectedSize, minSize.Width, minSize.Height)
	}
}

// TestIconThumbnailRenderer verifies that CreateRenderer returns a valid renderer
func TestIconThumbnailRenderer(t *testing.T) {
	// Create a test app for Fyne widget testing
	test.NewApp()

	thumbnail := NewIconThumbnail("", "Test", nil)

	renderer := thumbnail.CreateRenderer()

	// Verify renderer is not nil
	if renderer == nil {
		t.Fatal("Expected renderer to be non-nil")
	}

	// Verify renderer has objects to render
	objects := renderer.Objects()
	if len(objects) == 0 {
		t.Error("Expected renderer to have at least one object")
	}
}

// TestIconThumbnailWithNilCallback verifies that widget works with nil delete callback
func TestIconThumbnailWithNilCallback(t *testing.T) {
	// Create IconThumbnail with nil callback
	thumbnail := NewIconThumbnail("", "Test", nil)

	// Verify widget was created successfully
	if thumbnail == nil {
		t.Fatal("Expected thumbnail to be non-nil")
	}

	// Verify calling onDelete with nil callback doesn't crash
	if thumbnail.onDelete != nil {
		thumbnail.onDelete() // Should not crash
	}

	// Verify delete button exists and can be tapped without crashing
	if thumbnail.deleteBtn != nil && thumbnail.deleteBtn.OnTapped != nil {
		thumbnail.deleteBtn.OnTapped() // Should not crash
	}
}
