package assets

import (
	"Squire/internal/config"
	_ "embed"
	"log"
	"os"
	"path/filepath"
	"sync"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
)

//go:embed images/icon.svg
var appIcon []byte
var AppIcon = fyne.NewStaticResource("appIcon", appIcon)

var (
	//go:embed icons/double-up-chevron.svg
	doubleUpChevron []byte
	//go:embed icons/double-down-chevron.svg
	doubleDownChevron []byte
	//go:embed icons/chevron-up.svg
	chevronUp []byte
	//go:embed icons/chevron-down.svg
	chevronDown []byte
	//go:embed icons/mouse-click.svg
	mouseClick []byte
	//go:embed icons/mouse-click-filled.svg
	mouseClickFilled []byte
	//go:embed icons/mouse.svg
	mouse []byte
	//go:embed icons/image-search.svg
	imageSearch []byte
	//go:embed icons/text-search.svg
	textSearch []byte
	//go:embed icons/calculate.svg
	calculate []byte
	//go:embed icons/variable.svg
	variable []byte
)

var (
	DoubleUpChevronIcon   = fyne.NewStaticResource("doubleUpChevron", doubleUpChevron)
	DoubleDownChevronIcon = fyne.NewStaticResource("doubleDownChevron", doubleDownChevron)
	ChevronUpIcon         = fyne.NewStaticResource("chevronUp", chevronUp)
	ChevronDownIcon       = fyne.NewStaticResource("chevronDown", chevronDown)
	MouseClickIcon        = fyne.NewStaticResource("mouseClick", mouseClick)
	MouseClickFilledIcon  = fyne.NewStaticResource("mouseClickFilled", mouseClickFilled)
	MouseIcon             = fyne.NewStaticResource("mouse", mouse)
	ImageSearchIcon       = fyne.NewStaticResource("imageSearch", imageSearch)
	TextSearchIcon        = fyne.NewStaticResource("textSearch", textSearch)
	CalculateIcon         = fyne.NewStaticResource("calculate", calculate)
	VariableIcon          = fyne.NewStaticResource("variable", variable)
)

var (
	// fyneResourceCache stores loaded Fyne resources keyed by file path
	// Key format: "programName|filename.png"
	fyneResourceCache = make(map[string]*fyne.StaticResource)

	// fyneResourceMutex protects concurrent access to fyneResourceCache
	fyneResourceMutex sync.RWMutex

	// canvasImageCache stores decoded canvas.Image objects to prevent memory bloat
	// from repeatedly decoding the same PNG data
	canvasImageCache = make(map[string]*canvas.Image)

	// canvasImageMutex protects concurrent access to canvasImageCache
	canvasImageMutex sync.RWMutex
)

// GetFyneResource returns a single cached Fyne resource by key, loading from disk if not cached.
// This is more efficient than BytesToFyneIcons() when you only need one icon.
// Returns nil if the icon file doesn't exist or can't be loaded.
func GetFyneResource(key string) *fyne.StaticResource {
	fyneResourceMutex.RLock()
	resource, exists := fyneResourceCache[key]
	fyneResourceMutex.RUnlock()

	if exists {
		// log.Printf("DEBUG: Cache HIT for key: %s", key)
		return resource
	}

	// Not in cache, try to load from disk
	fyneResourceMutex.Lock()
	defer fyneResourceMutex.Unlock()

	// Double-check after acquiring write lock (another goroutine might have loaded it)
	if resource, exists := fyneResourceCache[key]; exists {
		// log.Printf("DEBUG: Cache HIT (after lock) for key: %s", key)
		return resource
	}

	// Parse key to get file path: "programName|filename.png"
	// Split on first delimiter to get program name and filename
	parts := splitOnFirstDelimiter(key, config.ProgramDelimiter)
	if len(parts) != 2 {
		log.Printf("Invalid cache key format: %s", key)
		return nil
	}

	programName := parts[0]
	filename := parts[1]

	// Construct file path
	iconsPath := config.GetIconsPath()
	iconPath := filepath.Join(iconsPath, programName, filename)

	// log.Printf("DEBUG: Loading from disk - key: %s, path: %s", key, iconPath)

	// Read icon file
	iconBytes, err := os.ReadFile(iconPath)
	if err != nil {
		if !os.IsNotExist(err) {
			log.Printf("Could not read icon %s. Error: %v", iconPath, err)
		}
		return nil
	}

	// Cache and return
	resource = fyne.NewStaticResource(key, iconBytes)
	fyneResourceCache[key] = resource
	// log.Printf("DEBUG: Cached resource for key: %s", key)
	return resource
}

// splitOnFirstDelimiter splits a string on the first occurrence of delimiter
func splitOnFirstDelimiter(s, delimiter string) []string {
	idx := len(delimiter)
	for i := 0; i <= len(s)-len(delimiter); i++ {
		if s[i:i+len(delimiter)] == delimiter {
			idx = i
			break
		}
	}
	if idx >= len(s) {
		return []string{s}
	}
	return []string{s[:idx], s[idx+len(delimiter):]}
}

// BytesToFyneIcons returns cached Fyne resources, loading from disk only on first call
// or after cache invalidation. When cache is empty, loads all icons. When cache has entries,
// scans filesystem and loads any new icons that aren't cached yet.
// NOTE: This returns a copy of the entire cache which can be memory-intensive.
// Consider using GetFyneResource() for individual icon lookups instead.
func BytesToFyneIcons() map[string]*fyne.StaticResource {
	fyneResourceMutex.Lock()
	defer fyneResourceMutex.Unlock()

	// If cache is completely empty, do a full load
	initialLoad := len(fyneResourceCache) == 0
	if initialLoad {
		log.Printf("Loading icons from disk and populating cache...")
	}

	iconsPath := config.GetIconsPath()

	// Read program directories from filesystem
	entries, err := os.ReadDir(iconsPath)
	if err != nil {
		// Graceful degradation if directory doesn't exist
		if os.IsNotExist(err) {
			log.Printf("Icons directory does not exist: %s", iconsPath)
			return make(map[string]*fyne.StaticResource)
		}
		log.Printf("Could not read directory %s. Error: %v", iconsPath, err)
		return make(map[string]*fyne.StaticResource)
	}

	// Scan filesystem and load any icons not in cache
	for _, entry := range entries {
		if entry.IsDir() {
			programName := entry.Name()
			programPath := filepath.Join(iconsPath, programName)

			subentries, err := os.ReadDir(programPath)
			if err != nil {
				log.Printf("Could not read directory %s. Error: %v", programPath, err)
				continue
			}

			for _, se := range subentries {
				if se.IsDir() {
					continue
				}

				// Construct cache key
				key := programName + config.ProgramDelimiter + se.Name()

				// Only load if not already in cache
				if _, exists := fyneResourceCache[key]; !exists {
					iconPath := filepath.Join(programPath, se.Name())
					iconBytes, err := os.ReadFile(iconPath)
					if err != nil {
						log.Printf("Could not read icon %s. Error: %v", iconPath, err)
						continue
					}

					fyneResourceCache[key] = fyne.NewStaticResource(key, iconBytes)
					if !initialLoad {
						log.Printf("Loaded new icon from disk: %s", key)
					}
				}
			}
		}
	}

	// Return a copy of the cache to prevent external modification
	result := make(map[string]*fyne.StaticResource, len(fyneResourceCache))
	for k, v := range fyneResourceCache {
		result[k] = v
	}
	return result
}

// InvalidateFyneResourceCache removes a specific icon from the cache
// Called by IconVariantService after add/delete operations
func InvalidateFyneResourceCache(key string) {
	fyneResourceMutex.Lock()
	canvasImageMutex.Lock()
	defer fyneResourceMutex.Unlock()
	defer canvasImageMutex.Unlock()

	delete(fyneResourceCache, key)
	delete(canvasImageCache, key)
	log.Printf("Invalidated cache entry for key: %s", key)
}

// ClearFyneResourceCache removes all entries from the cache
// Useful for testing or full cache reset
func ClearFyneResourceCache() {
	fyneResourceMutex.Lock()
	canvasImageMutex.Lock()
	defer fyneResourceMutex.Unlock()
	defer canvasImageMutex.Unlock()

	fyneResourceCache = make(map[string]*fyne.StaticResource)
	canvasImageCache = make(map[string]*canvas.Image)
	log.Printf("Cleared all cache entries")
}

// GetCanvasImage returns a cached canvas.Image for the given key, creating it if necessary.
// The caller should set the desired minSize and fillMode on the returned image.
func GetCanvasImage(key string, minSize fyne.Size, fillMode canvas.ImageFill) *canvas.Image {
	// Check canvas image cache first
	canvasImageMutex.RLock()
	if img, exists := canvasImageCache[key]; exists {
		canvasImageMutex.RUnlock()
		// Create a copy with the requested settings to avoid modifying the cached version
		newImg := canvas.NewImageFromResource(img.Resource)
		newImg.FillMode = fillMode
		newImg.SetMinSize(minSize)
		return newImg
	}
	canvasImageMutex.RUnlock()

	// Get the Fyne resource
	resource := GetFyneResource(key)
	if resource == nil {
		return nil
	}

	// Create canvas image with write lock
	canvasImageMutex.Lock()
	defer canvasImageMutex.Unlock()

	// Double-check after acquiring write lock
	if img, exists := canvasImageCache[key]; exists {
		// Create a copy with the requested settings
		newImg := canvas.NewImageFromResource(img.Resource)
		newImg.FillMode = fillMode
		newImg.SetMinSize(minSize)
		return newImg
	}

	// Create new canvas.Image from resource
	img := canvas.NewImageFromResource(resource)

	// Cache the base image without specific size/fill settings
	canvasImageCache[key] = img

	// Return a copy with the requested settings
	newImg := canvas.NewImageFromResource(resource)
	newImg.FillMode = fillMode
	newImg.SetMinSize(minSize)
	return newImg
}
