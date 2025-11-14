# Implementation Plan

- [x] 1. Implement Fyne Resource cache in internal/assets/embeds.go
  - Add package-level cache variables (fyneResourceCache map and fyneResourceMutex)
  - Modify `BytesToFyneIcons()` to use cache instead of reloading from disk
  - Implement `InvalidateFyneResourceCache(key string)` function
  - Implement `ClearFyneResourceCache()` function for testing
  - Remove `LoadIconBytes()` and `GetIconBytes()` functions
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 2. Update IconThumbnail widget to use cached Fyne Resources
  - Modify `loadIcon()` to call `assets.BytesToFyneIcons()` and use `canvas.NewImageFromResource()`
  - Remove direct `canvas.NewImageFromFile()` calls
  - Remove file existence checks (handled by cache layer)
  - Add helper function to construct icon key from file path
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 3.1, 3.2_

- [x] 3. Integrate cache invalidation in IconVariantService
  - Modify `AddVariant()` to call `assets.InvalidateFyneResourceCache()` after successful file copy
  - Modify `DeleteVariant()` to call `assets.InvalidateFyneResourceCache()` after successful file deletion
  - Add helper function to construct cache key from program name and filename
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 4. Update binders/item.go to use cached resources
  - Modify accordion item rendering to use `assets.BytesToFyneIcons()` once per render cycle
  - Remove redundant calls to icon loading functions
  - Ensure icon display uses cached Fyne Resources
  - _Requirements: 3.3, 3.4_

- [x] 5. Write unit tests for Fyne Resource cache
  - Test cache hit on repeated `BytesToFyneIcons()` calls
  - Test cache invalidation removes specific entry
  - Test cache clear removes all entries
  - Test concurrent access with multiple goroutines (race condition test)
  - Test cache miss loads from disk and stores in cache
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 5.1, 5.2, 5.3_

- [x] 6. Write unit tests for IconThumbnail widget
  - Test `loadIcon()` creates canvas.Image from cached Fyne Resource
  - Test multiple IconThumbnail instances create independent canvas.Image objects
  - Test invalid icon path returns placeholder
  - Test icon loading after cache invalidation reloads from disk
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 7. Write integration tests for cache invalidation
  - Test `AddVariant()` invalidates Fyne Resource cache
  - Test `DeleteVariant()` invalidates Fyne Resource cache
  - Test end-to-end flow: load icon → add variant → verify cache invalidated → verify reload
  - Test end-to-end flow: load icon → delete variant → verify cache invalidated → verify placeholder
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
