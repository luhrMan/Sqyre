# Implementation Plan

## Overview
This implementation plan covers the icon variants feature, which allows users to manage multiple visual representations of items through filesystem-based PNG files using the ProgramDelimiter naming convention.

## Tasks

- [x] 1. Create Icon Variant Service
  - Implement `internal/services/iconVariants.go` with filesystem operations for discovering, validating, and managing icon variant files
  - Implement `GetVariants(programName, itemName string) ([]string, error)` to discover variants by scanning filesystem
  - Implement `GetVariantPath(programName, itemName, variantName string) string` to construct full paths to variant icons
  - Implement `AddVariant(programName, itemName, variantName, sourcePath string) error` to copy files with proper naming
  - Implement `DeleteVariant(programName, itemName, variantName string) error` to remove variant files
  - Implement `GetBaseItemName(fullItemName string) string` to extract base name by parsing text before ProgramDelimiter
  - Implement `ValidateVariantFile(path string) error` to check PNG file validity using header checks
  - Use `filepath.Glob()` for pattern matching `{ItemName}|*.png`
  - Return sorted variant lists for consistent UI display
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 2.3, 3.1, 3.4_

- [x] 1.1 Write unit tests for Icon Variant Service
  - Create `internal/services/iconVariants_test.go`
  - Test `GetVariants` with various filesystem scenarios
  - Test `GetBaseItemName` with and without delimiter
  - Test `AddVariant` file copy operations
  - Test `DeleteVariant` file removal
  - Test `ValidateVariantFile` PNG validation
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 2.3_

- [x] 2. Create Icon Thumbnail Widget
  - Implement `ui/custom_widgets/iconThumbnail.go` as a custom Fyne widget
  - Create struct with `iconPath`, `variantName`, `onDelete` callback fields
  - Load PNG using `canvas.NewImageFromFile()` with 64x64 pixel sizing
  - Scale images maintaining aspect ratio using `canvas.ImageFillContain`
  - Display placeholder with error indicator for missing/corrupted files
  - Add variant name label below thumbnail
  - Add delete button with danger styling
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 3. Create Icon Variant Editor Widget
  - Implement `ui/custom_widgets/iconVariantEditor.go` as a custom Fyne widget
  - Create struct with `programName`, `itemName`, `variants`, `service` fields
  - Display grid of IconThumbnail widgets for existing variants
  - Implement "Add Icon Variant" button that opens file picker dialog
  - Prompt user for variant name using dialog when adding new icon
  - Implement confirmation dialog before deletion
  - Prevent deletion when only one variant remains (disable delete button)
  - Refresh display after add/delete operations using `onVariantChange` callback
  - Wire up IconVariantService for all file operations
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 4. Update Item Grid to Group Variants
  - Modify `binders/item.go` `setAccordionItemsLists()` function
  - Use `GetBaseItemName()` to extract base names from all items
  - Group items with same base name to prevent duplicate entries in filtered list
  - Display only one gridwrap item per base name in accordion
  - Load first variant's icon for gridwrap item display
  - Maintain alphabetical sorting by base item name
  - Update selection handler to load base item name and discover all variants
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 3.4, 3.5_

- [x] 5. Integrate Icon Variant Editor into Item Editor Panel
  - Modify `ui/editor.go` `constructEditorTabs()` function
  - Add IconVariantEditor widget to Items tab form after StackMax field
  - Create form item with label "Icon Variants"
  - Initialize IconVariantEditor with selected program name and item name
  - Wire up variant change callback to refresh accordion items
  - Update `binders/editor.go` to initialize IconVariantEditor when item is selected
  - _Requirements: 1.2, 4.1, 4.2_

- [x] 6. Update Icon Loading to Support Variants
  - Modify `internal/assets/embeds.go` to handle variant naming convention
  - Update `LoadIconBytes()` to recognize files with ProgramDelimiter in names
  - Ensure `BytesToFyneIcons()` correctly maps variant files
  - Update icon path construction in `binders/item.go` to use first variant when multiple exist
  - Maintain backward compatibility with non-variant icon files
  - _Requirements: 1.1, 3.4, 3.5_

- [x] 7. Add Constants for Icon Variants
  - Add `IconsPath = ImagesPath + "icons/"` to `internal/config/constants.go`
  - Add `IconThumbnailSize = 64` constant for consistent thumbnail sizing
  - Add `MaxIconVariants = 100` constant for variant limit validation
  - _Requirements: 1.4, 6.1, 6.4_

- [x] 8. Implement Error Handling and Validation
  - Add path traversal protection in `AddVariant()` using `filepath.Clean()`
  - Validate PNG header (first 8 bytes: `\x89PNG\r\n\x1a\n`) in `ValidateVariantFile()`
  - Check variant count before add operation (max 100 per item)
  - Show error dialogs for: insufficient permissions, duplicate variant names, invalid PNG files
  - Handle missing icon files gracefully with placeholder display
  - _Requirements: 1.3, 1.4, 1.5_

- [x] 8.1 Write integration tests
  - Create tests in `internal/models/repositories/integration_test.go`
  - Test end-to-end variant management: create item, add variants, verify filesystem
  - Test delete variant and verify file removed
  - Test loading item with variants and verify all discovered
  - Test with multiple programs simultaneously
  - _Requirements: 1.1, 1.5, 3.1, 3.2, 3.3_

## Notes

- Icon variants are filesystem artifacts, not data model properties - no changes to Item struct required
- The ProgramDelimiter ("|") is already defined in constants and used throughout the codebase
- Existing icon loading infrastructure in `internal/assets/embeds.go` needs extension, not replacement
- The accordion population logic in `binders/item.go` already handles program-specific items
- File picker in Fyne doesn't support filters on all platforms - validate file type after selection
- Tasks marked with * are optional and focus on testing rather than core functionality
