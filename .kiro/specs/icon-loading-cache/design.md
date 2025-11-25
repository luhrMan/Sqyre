# Icon Loading Cache - Design Document

## Overview

This design implements a two-tier caching system for icon loading in Squire to eliminate redundant disk I/O and improve UI performance. The system caches both Fyne Resources (at the package level in `internal/assets/embeds.go`) and Canvas Images (in `ui/custom_widgets/iconThumbnail.go`), with thread-safe operations and cache invalidation support.

### Current State

The current implementation loads icons from disk repeatedly without caching:

1. `BytesToFyneIcons()` in `internal/assets/embeds.go` calls `LoadIconBytes()` every time, reading all icon files from disk
2. `IconThumbnail.loadIcon()` creates a new `canvas.Image` from file for each widget instance
3. Multiple accordion items and variant editors load the same icons repeatedly
4. No cache invalidation mechanism exists for filesystem changes

### Target State

The new implementation will:

1. Cache Fyne Resources at package level to avoid repeated disk reads
2. Cache Canvas Images to avoid repeated image object creation
3. Provide thread-safe cache operations using mutexes
4. Support cache invalidation when icons are added or deleted
5. Remove all non-caching icon loading code

## Architecture

### Single-Tier Cache Design

```
┌─────────────────────────────────────────────────────────────┐
│                     Application Layer                        │
│  (binders/item.go, UI components)                           │
│  Each widget creates its own canvas.Image                   │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌────────────────────────────────────┐
│   Fyne Resource Cache              │
│   (internal/assets/embeds.go)      │
│                                    │
│  - Package-level cache             │
│  - Maps file path → fyne.Resource  │
│  - Thread-safe with mutex          │
│  - Lazy loading on first access    │
│  - Invalidation support            │
└────────────────────┬───────────────┘
                     │
                     ▼
                ┌────────────────────────┐
                │   Filesystem           │
                │   ~/Sqyre/images/icons/│
                └────────────────────────┘
```

### Cache Interaction Flow

1. **Initial Load**: UI requests icon → Check Fyne Resource cache → Miss → Load from disk → Store in cache → Create canvas.Image from cached resource
2. **Subsequent Load**: UI requests icon → Check Fyne Resource cache → Hit → Create canvas.Image from cached resource
3. **Variant Add/Delete**: IconVariantService modifies filesystem → Invalidate cache for affected path → Next access reloads from disk

### Why No Canvas Image Cache?

Canvas.Image objects cannot be safely shared across multiple widget instances because:
- Each widget needs independent rendering state (position, size, visibility)
- Fyne's rendering engine expects each widget to own its canvas objects
- Sharing would cause rendering conflicts and state corruption

The Fyne Resource cache provides the primary performance benefit by eliminating disk I/O (the expensive operation). Creating canvas.Image objects from cached resources is fast and memory-efficient.

## Components and Interfaces

### 1. Fyne Resource Cache (`internal/assets/embeds.go`)

**New Package-Level Variables:**
```go
var (
    // fyneResourceCache stores loaded Fyne resources keyed by file path
    // Key format: "programName|filename.png"
    fyneResourceCache = make(map[string]*fyne.StaticResource)
    
    // fyneResourceMutex protects concurrent access to fyneResourceCache
    fyneResourceMutex sync.RWMutex
)
```

**Modified Functions:**

```go
// BytesToFyneIcons returns cached Fyne resources, loading from disk only on first call
// or after cache invalidation
func BytesToFyneIcons() map[string]*fyne.StaticResource

// InvalidateFyneResourceCache removes a specific icon from the cache
// Called by IconVariantService after add/delete operations
func InvalidateFyneResourceCache(key string)

// ClearFyneResourceCache removes all entries from the cache
// Useful for testing or full cache reset
func ClearFyneResourceCache()
```

**Removed Functions:**
- `LoadIconBytes()` - Replaced by internal caching logic in `BytesToFyneIcons()`
- `GetIconBytes()` - No longer needed with caching

### 2. Icon Thumbnail Widget (`ui/custom_widgets/iconThumbnail.go`)

**Modified Methods:**

```go
// loadIcon now creates canvas.Image from cached Fyne Resource
// instead of loading directly from file
func (t *IconThumbnail) loadIcon() *canvas.Image {
    // Get cached Fyne Resource from assets package
    icons := assets.BytesToFyneIcons()
    
    // Construct key: programName|filename
    key := constructIconKey(t.iconPath)
    
    if resource, exists := icons[key]; exists {
        // Create new canvas.Image from cached resource
        img := canvas.NewImageFromResource(resource)
        img.FillMode = canvas.ImageFillContain
        img.SetMinSize(fyne.NewSize(config.IconThumbnailSize, config.IconThumbnailSize))
        return img
    }
    
    // Fallback to placeholder if not found
    return t.createPlaceholder(true)
}
```

**Removed Code:**
- Direct `canvas.NewImageFromFile()` calls - Replaced with `canvas.NewImageFromResource()` using cached resources
- File existence checks - Handled by cache layer

### 3. Icon Variant Service Integration (`internal/services/iconVariants.go`)

**Modified Functions:**

```go
// AddVariant now invalidates Fyne Resource cache after successful file copy
func (s *IconVariantService) AddVariant(programName, itemName, variantName, sourcePath string) error

// DeleteVariant now invalidates Fyne Resource cache after successful file deletion
func (s *IconVariantService) DeleteVariant(programName, itemName, variantName string) error
```

**Cache Invalidation Logic:**
```go
// After successful add/delete:
// 1. Construct cache key: programName + "|" + filename
// 2. Call assets.InvalidateFyneResourceCache(key)
```

## Data Models

### Cache Key Format

**Fyne Resource Cache Key:**
```
Format: "programName|filename.png"
Examples:
  - "dark and darker|Health Potion.png"
  - "dark and darker|Health Potion|Variant1.png"
  - "path of exile 2|Scroll of Wisdom.png"
```

### Cache Entry Structure

**Fyne Resource Cache:**
```go
map[string]*fyne.StaticResource
// Key: "programName|filename.png"
// Value: Pointer to StaticResource (immutable, can be safely shared across widgets)
```

## Error Handling

### Cache Miss Scenarios

1. **Icon file doesn't exist**: Return placeholder/broken icon, don't cache
2. **Invalid PNG file**: Return placeholder/broken icon, don't cache
3. **File read error**: Log error, return placeholder, don't cache
4. **Concurrent access**: Mutex ensures thread safety, no special handling needed

### Cache Invalidation Scenarios

1. **Variant added**: Invalidate Fyne Resource cache for new file path
2. **Variant deleted**: Invalidate Fyne Resource cache for deleted file path
3. **File doesn't exist during invalidation**: No-op (idempotent)
4. **Invalid cache key**: No-op (key not found)

### Thread Safety

All cache operations use `sync.RWMutex`:
- **Read operations** (cache lookup): Use `RLock()` / `RUnlock()`
- **Write operations** (cache insert/delete): Use `Lock()` / `Unlock()`
- **Lazy initialization**: First write operation initializes cache if needed

## Testing Strategy

### Unit Tests

**Fyne Resource Cache Tests** (`internal/assets/embeds_test.go`):
1. Test cache hit on repeated `BytesToFyneIcons()` calls
2. Test cache invalidation removes specific entry
3. Test cache clear removes all entries
4. Test concurrent access doesn't cause race conditions
5. Test cache miss loads from disk and stores in cache

**Icon Thumbnail Tests** (`ui/custom_widgets/iconThumbnail_test.go`):
1. Test `loadIcon()` creates canvas.Image from cached Fyne Resource
2. Test multiple IconThumbnail instances create independent canvas.Image objects
3. Test invalid icon path returns placeholder
4. Test icon loading after cache invalidation reloads from disk

**Icon Variant Service Tests** (`internal/services/iconVariants_test.go`):
1. Test `AddVariant()` invalidates Fyne Resource cache
2. Test `DeleteVariant()` invalidates Fyne Resource cache
3. Test cache invalidation with non-existent files (no-op)

### Integration Tests

**End-to-End Cache Flow** (`internal/services/iconVariants_integration_test.go`):
1. Load icon variant → Verify cached in Fyne Resource cache
2. Add new variant → Verify cache invalidated → Verify reload from disk
3. Delete variant → Verify cache invalidated → Verify placeholder shown
4. Load multiple variants → Verify each cached independently
5. Create multiple IconThumbnail widgets → Verify each creates own canvas.Image from shared resource

### Performance Tests

**Benchmark Tests**:
1. Benchmark icon loading with cache vs without cache
2. Benchmark concurrent cache access with multiple goroutines
3. Measure memory usage with 100+ cached icons
4. Measure cache invalidation performance

### Manual Testing

1. Open variant editor → Verify fast loading
2. Add new variant → Verify appears immediately
3. Delete variant → Verify removed immediately
4. Open multiple accordion items → Verify no lag
5. Monitor memory usage during extended use

## Implementation Notes

### Cache Initialization

The cache uses lazy initialization:
- Cache is initialized as an empty map at package level
- First access populates cache entries on-demand
- No explicit initialization function needed

### Memory Management

- Fyne Resources are immutable and can be safely shared across widgets
- Each widget creates its own canvas.Image from the shared resource (lightweight operation)
- No explicit cleanup needed (Go GC handles unused entries)
- Cache size is bounded by number of unique icon files (typically < 1000)
- Memory usage: ~1-5 MB for 100 icons (PNG data in memory)

### Backward Compatibility

This design **removes** backward compatibility:
- All non-caching code is deleted
- Direct file loading is replaced with cache-based loading
- Calling code in `binders/item.go` continues to work without changes (uses same public APIs)

### Performance Expectations

**Before Caching:**
- Loading 50 icon variants: ~500ms (10ms per icon × 50 disk reads)
- Memory: 50 duplicate Fyne Resources loaded from disk repeatedly

**After Caching:**
- First load: ~500ms (same as before, populates cache)
- Subsequent loads: ~5ms (cache lookup + canvas.Image creation)
- Memory: 50 unique Fyne Resources (shared), each widget creates lightweight canvas.Image

**Expected Improvements:**
- 100x faster icon loading after initial cache population
- Eliminates redundant disk I/O (primary bottleneck)
- Instant variant editor opening (< 10ms)
- Reduced memory usage from eliminating duplicate Fyne Resources
