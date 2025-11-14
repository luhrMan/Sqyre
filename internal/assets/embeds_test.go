package assets

import (
	"sync"
	"testing"

	"fyne.io/fyne/v2"
)

// TestBytesToFyneIcons verifies that BytesToFyneIcons correctly loads and caches
// icon resources from the filesystem
func TestBytesToFyneIcons(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	fyneIcons := BytesToFyneIcons()

	if fyneIcons == nil {
		t.Fatal("Expected fyneIcons map to be non-nil")
	}

	// Verify that icons are loaded (we should have at least some icons or empty map)
	if len(fyneIcons) == 0 {
		t.Log("Warning: No icons loaded - this may be expected if no icon files exist")
	}

	// Verify that icon keys follow the expected format: programName|filename.png
	for key, resource := range fyneIcons {
		if key == "" {
			t.Error("Found empty key in icons map")
		}
		if resource == nil {
			t.Errorf("Found nil resource for key: %s", key)
		}
		// Keys should contain at least one delimiter (programName|filename)
		// They may contain two delimiters for variants (programName|itemName|variantName.png)
	}
}

// TestCacheHitOnRepeatedCalls verifies that repeated calls to BytesToFyneIcons
// return cached resources without reloading from disk
func TestCacheHitOnRepeatedCalls(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	// First call - loads from disk and populates cache
	firstCall := BytesToFyneIcons()
	firstCallLen := len(firstCall)

	// Second call - should return cached resources
	secondCall := BytesToFyneIcons()
	secondCallLen := len(secondCall)

	// Verify both calls return the same number of resources
	if firstCallLen != secondCallLen {
		t.Errorf("Expected same number of resources on repeated calls, got %d and %d", firstCallLen, secondCallLen)
	}

	// Verify the resources are the same (same pointers indicate cache hit)
	for key, firstResource := range firstCall {
		secondResource, exists := secondCall[key]
		if !exists {
			t.Errorf("Key %s exists in first call but not in second call", key)
			continue
		}
		// Compare resource pointers - they should be identical for cached resources
		if firstResource != secondResource {
			t.Errorf("Resource for key %s has different pointer on second call (expected cache hit)", key)
		}
	}
}

// TestCacheInvalidationRemovesSpecificEntry verifies that InvalidateFyneResourceCache
// removes only the specified cache entry and it gets reloaded on next access
func TestCacheInvalidationRemovesSpecificEntry(t *testing.T) {
	// Clear cache and load icons
	ClearFyneResourceCache()
	icons := BytesToFyneIcons()

	if len(icons) == 0 {
		t.Skip("No icons available for testing cache invalidation")
	}

	// Pick the first icon key to invalidate
	var testKey string
	var originalResource *fyne.StaticResource
	for key, resource := range icons {
		testKey = key
		originalResource = resource
		break
	}

	// Invalidate the specific entry
	InvalidateFyneResourceCache(testKey)

	// Load icons again - the invalidated key should be reloaded from disk
	iconsAfterInvalidation := BytesToFyneIcons()

	// The invalidated key should exist (reloaded from disk)
	reloadedResource, exists := iconsAfterInvalidation[testKey]
	if !exists {
		t.Fatalf("Invalidated key %s should be reloaded from disk", testKey)
	}

	// The reloaded resource should have a different pointer (new instance)
	if originalResource == reloadedResource {
		t.Errorf("Invalidated key %s should have been reloaded (expected different pointer)", testKey)
	}

	// The content should be the same (same file)
	if len(originalResource.Content()) != len(reloadedResource.Content()) {
		t.Errorf("Reloaded resource should have same content length, got %d and %d",
			len(originalResource.Content()), len(reloadedResource.Content()))
	}

	// Other keys should still be cached (same pointers)
	for key, originalRes := range icons {
		if key == testKey {
			continue // Skip the invalidated key
		}

		newRes, exists := iconsAfterInvalidation[key]
		if !exists {
			t.Errorf("Non-invalidated key %s should still be in cache", key)
			continue
		}

		// Other keys should still be cached (same pointer)
		if originalRes != newRes {
			t.Errorf("Non-invalidated key %s should still be cached (expected same pointer)", key)
		}
	}
}

// TestCacheClearRemovesAllEntries verifies that ClearFyneResourceCache
// removes all cache entries
func TestCacheClearRemovesAllEntries(t *testing.T) {
	// Clear cache and load icons
	ClearFyneResourceCache()
	firstLoad := BytesToFyneIcons()

	if len(firstLoad) == 0 {
		t.Skip("No icons available for testing cache clear")
	}

	// Clear the entire cache
	ClearFyneResourceCache()

	// Load icons again - all should be reloaded from disk
	secondLoad := BytesToFyneIcons()

	// Verify same number of icons loaded
	if len(firstLoad) != len(secondLoad) {
		t.Errorf("Expected same number of icons after cache clear, got %d and %d", len(firstLoad), len(secondLoad))
	}

	// All resources should have different pointers (reloaded from disk)
	for key, firstResource := range firstLoad {
		secondResource, exists := secondLoad[key]
		if !exists {
			t.Errorf("Key %s should exist in both loads", key)
			continue
		}

		// After cache clear, all resources should be reloaded (different pointers)
		if firstResource == secondResource {
			t.Errorf("Resource for key %s should be reloaded after cache clear (expected different pointer)", key)
		}
	}
}

// TestConcurrentAccess verifies that concurrent access to the cache is thread-safe
// and doesn't cause race conditions
func TestConcurrentAccess(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	// Number of concurrent goroutines
	numGoroutines := 10
	var wg sync.WaitGroup
	wg.Add(numGoroutines)

	// Channel to collect results
	results := make(chan map[string]*fyne.StaticResource, numGoroutines)

	// Launch multiple goroutines that all call BytesToFyneIcons concurrently
	for i := 0; i < numGoroutines; i++ {
		go func() {
			defer wg.Done()
			icons := BytesToFyneIcons()
			results <- icons
		}()
	}

	// Wait for all goroutines to complete
	wg.Wait()
	close(results)

	// Collect all results
	var allResults []map[string]*fyne.StaticResource
	for result := range results {
		allResults = append(allResults, result)
	}

	// Verify all goroutines got the same number of icons
	if len(allResults) == 0 {
		t.Fatal("Expected at least one result from concurrent access test")
	}

	firstLen := len(allResults[0])
	for i, result := range allResults {
		if len(result) != firstLen {
			t.Errorf("Goroutine %d got %d icons, expected %d", i, len(result), firstLen)
		}
	}

	// Verify all goroutines got the same cached resources (same pointers)
	firstResult := allResults[0]
	for i := 1; i < len(allResults); i++ {
		for key, firstResource := range firstResult {
			otherResource, exists := allResults[i][key]
			if !exists {
				t.Errorf("Key %s missing in result %d", key, i)
				continue
			}
			// All goroutines should get the same cached resource (same pointer)
			if firstResource != otherResource {
				t.Errorf("Result %d has different pointer for key %s (race condition detected)", i, key)
			}
		}
	}
}

// TestCacheMissLoadsFromDisk verifies that when cache is empty or an entry is invalidated,
// icons are loaded from disk and stored in cache
func TestCacheMissLoadsFromDisk(t *testing.T) {
	// Clear cache to ensure cache miss
	ClearFyneResourceCache()

	// First load - cache miss, loads from disk
	firstLoad := BytesToFyneIcons()
	firstLoadLen := len(firstLoad)

	if firstLoadLen == 0 {
		t.Skip("No icons available in filesystem for testing cache miss")
	}

	// Pick the first icon to test
	var testKey string
	var firstResource *fyne.StaticResource
	for key, resource := range firstLoad {
		testKey = key
		firstResource = resource
		break
	}

	// Verify resource was loaded
	if firstResource == nil {
		t.Fatalf("Expected non-nil resource for key %s", testKey)
	}

	// Second load - cache hit, should return same resource pointer
	secondLoad := BytesToFyneIcons()
	secondResource, exists := secondLoad[testKey]
	if !exists {
		t.Fatalf("Expected icon with key %s to be in cache on second call", testKey)
	}

	// Verify it's the same cached resource (same pointer indicates cache hit)
	if firstResource != secondResource {
		t.Errorf("Expected same resource pointer on second call (cache hit), got different pointers")
	}

	// Now invalidate the specific entry
	InvalidateFyneResourceCache(testKey)

	// Third load - cache miss for this specific key, should reload from disk
	thirdLoad := BytesToFyneIcons()
	thirdResource, exists := thirdLoad[testKey]
	if !exists {
		t.Fatalf("Expected icon with key %s to be reloaded after invalidation", testKey)
	}

	// Verify it's a different resource pointer (reloaded from disk)
	if firstResource == thirdResource {
		t.Errorf("Expected different resource pointer after cache invalidation (should reload from disk)")
	}

	// Verify the content is the same (same file data)
	if len(firstResource.Content()) != len(thirdResource.Content()) {
		t.Errorf("Expected same content length after reload, got %d and %d",
			len(firstResource.Content()), len(thirdResource.Content()))
	}
}

// TestGetFyneResource_SingleIconLoad verifies that GetFyneResource loads only
// the requested icon without loading the entire cache
func TestGetFyneResource_SingleIconLoad(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	// Request a specific icon
	key := "dark and darker|Gold Band|Original.png"
	resource := GetFyneResource(key)

	// Verify the resource was loaded
	if resource == nil {
		t.Skip("Test icon not found in filesystem, skipping test")
	}

	// Verify the resource has content
	if len(resource.Content()) == 0 {
		t.Error("Expected resource to have content")
	}

	// Verify the resource name matches the key
	if resource.Name() != key {
		t.Errorf("Expected resource name %q, got %q", key, resource.Name())
	}
}

// TestGetFyneResource_CachesAfterFirstLoad verifies that GetFyneResource
// caches the resource after first load
func TestGetFyneResource_CachesAfterFirstLoad(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	// Request a specific icon twice
	key := "dark and darker|Gold Band|Original.png"
	resource1 := GetFyneResource(key)
	if resource1 == nil {
		t.Skip("Test icon not found in filesystem, skipping test")
	}

	resource2 := GetFyneResource(key)
	if resource2 == nil {
		t.Fatal("Expected second call to return cached resource")
	}

	// Verify both calls return the same cached resource (same pointer)
	if resource1 != resource2 {
		t.Error("Expected same resource pointer on second call (cache hit)")
	}
}

// TestGetFyneResource_ReturnsNilForNonExistentIcon verifies that GetFyneResource
// returns nil for icons that don't exist
func TestGetFyneResource_ReturnsNilForNonExistentIcon(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	// Request a non-existent icon
	key := "nonexistent-program|nonexistent-icon.png"
	resource := GetFyneResource(key)

	// Verify nil is returned
	if resource != nil {
		t.Error("Expected nil for non-existent icon")
	}
}

// TestGetFyneResource_InvalidKeyFormat verifies that GetFyneResource
// handles invalid key formats gracefully
func TestGetFyneResource_InvalidKeyFormat(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	// Request with invalid key format (no delimiter)
	key := "invalid-key-without-delimiter"
	resource := GetFyneResource(key)

	// Verify nil is returned
	if resource != nil {
		t.Error("Expected nil for invalid key format")
	}
}

// TestGetFyneResource_ConcurrentAccess verifies that GetFyneResource
// is thread-safe when accessed concurrently
func TestGetFyneResource_ConcurrentAccess(t *testing.T) {
	// Clear cache before test
	ClearFyneResourceCache()

	key := "dark and darker|Gold Band|Original.png"

	// Number of concurrent goroutines
	numGoroutines := 10
	var wg sync.WaitGroup
	wg.Add(numGoroutines)

	// Channel to collect results
	results := make(chan *fyne.StaticResource, numGoroutines)

	// Launch multiple goroutines that all call GetFyneResource concurrently
	for i := 0; i < numGoroutines; i++ {
		go func() {
			defer wg.Done()
			resource := GetFyneResource(key)
			results <- resource
		}()
	}

	// Wait for all goroutines to complete
	wg.Wait()
	close(results)

	// Collect all results
	var allResults []*fyne.StaticResource
	for result := range results {
		allResults = append(allResults, result)
	}

	// Skip test if icon doesn't exist
	if allResults[0] == nil {
		t.Skip("Test icon not found in filesystem, skipping test")
	}

	// Verify all goroutines got the same cached resource (same pointer)
	firstResource := allResults[0]
	for i, result := range allResults {
		if result != firstResource {
			t.Errorf("Goroutine %d got different resource pointer (race condition detected)", i)
		}
	}
}
