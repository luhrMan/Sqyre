# Requirements Document

## Introduction

This feature implements a caching system for icon loading in Squire to improve performance and reduce memory usage. Currently, icons are loaded from disk repeatedly without caching, causing slow UI rendering (especially in the variant editor) and high memory consumption. The caching system will store loaded icons in memory and reuse them across the application, significantly improving performance when displaying icon thumbnails and accordion items.

## Glossary

- **Icon Cache**: An in-memory data structure that stores loaded icon resources to avoid repeated disk reads and conversions
- **Icon System**: The subsystem responsible for loading and managing icon image resources used in the UI
- **Fyne Resource**: A `fyne.StaticResource` object that represents an image resource in the Fyne UI framework
- **Canvas Image**: A `canvas.Image` object that displays an image in the Fyne UI
- **Icon Thumbnail**: A custom widget (`IconThumbnail`) that displays icon variants with preview, label, and delete button
- **Accordion Items**: UI components in the items list that display program-specific items with icons
- **Base Item Name**: The item name without variant suffix (e.g., "Health Potion" from "Health Potion|Variant1")
- **Program Delimiter**: The `|` character used to separate program name from item name in icon keys

## Requirements

### Requirement 1

**User Story:** As a user, I want icon variant editors to load quickly, so that I can efficiently manage my item icons

#### Acceptance Criteria

1. WHEN the Icon Thumbnail loads an icon, THE Icon System SHALL retrieve the cached Fyne Resource if the icon key exists in cache
2. WHEN a Fyne Resource is not in cache, THE Icon System SHALL load the icon from disk and store it in cache keyed by icon path
3. WHEN the Icon Thumbnail creates a canvas image, THE Icon System SHALL use `canvas.NewImageFromResource()` with the cached Fyne Resource
4. THE Icon System SHALL allow multiple Icon Thumbnail widget instances to share the same cached Fyne Resource

### Requirement 2

**User Story:** As a user, I want the application to use less memory, so that it runs efficiently on my system

#### Acceptance Criteria

1. THE Icon System SHALL load each unique icon file from disk at most once during application runtime
2. THE Icon System SHALL cache Fyne Resources at the package level in `internal/assets/embeds.go`
3. WHEN `BytesToFyneIcons()` is called multiple times, THE Icon System SHALL return the same cached map without reloading from disk
4. THE Icon System SHALL NOT create duplicate Fyne Resources for the same icon file path

### Requirement 3

**User Story:** As a developer, I want to eliminate redundant icon loading code, so that the codebase is cleaner and more maintainable

#### Acceptance Criteria

1. THE Icon System SHALL remove all non-caching icon loading code from `internal/assets/embeds.go`
2. THE Icon System SHALL remove all non-caching icon loading code from `ui/custom_widgets/iconThumbnail.go`
3. THE Icon System SHALL replace direct file loading calls with cache-based loading calls
4. THE Icon System SHALL remove duplicate icon loading logic across the codebase

### Requirement 4

**User Story:** As a user, I want newly added icon variants to appear immediately, so that I can see my changes without restarting

#### Acceptance Criteria

1. WHEN `IconVariantService.AddVariant()` completes successfully, THE Icon System SHALL invalidate cache entries for the affected icon file path
2. WHEN `IconVariantService.DeleteVariant()` completes successfully, THE Icon System SHALL invalidate cache entries for the affected icon file path
3. THE Icon System SHALL provide a cache invalidation method accepting a file path parameter
4. WHEN a cache entry is invalidated, THE Icon System SHALL reload the icon from disk on the next access request

### Requirement 5

**User Story:** As a developer, I want the cache to be thread-safe, so that concurrent icon loading operations don't cause data corruption

#### Acceptance Criteria

1. THE Icon System SHALL use mutex locks to protect cache read and write operations
2. THE Icon System SHALL prevent race conditions when multiple goroutines access the cache simultaneously
3. THE Icon System SHALL ensure cache operations are atomic and consistent
4. THE Icon System SHALL handle concurrent cache invalidation requests safely
