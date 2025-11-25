# Design Document: Item Icon Variants

## Overview

This feature extends the existing Item management system to support multiple icon variants per item without modifying the core Item data structure. Icon variants are stored as separate PNG files using a naming convention with the ProgramDelimiter ("|") character, and managed through the existing Editor UI with new variant management controls.

The design maintains backward compatibility with existing single-icon items while enabling users to add, remove, and preview multiple visual representations of the same logical item.

## Architecture

### High-Level Design

The icon variants feature follows a filesystem-based approach where:
- Item data remains unchanged in the YAML configuration
- Icon variants are stored as separate PNG files in `internal/assets/images/icons/{ProgramName}/`
- File naming convention: `{ItemName}|{VariantName}.png`
- UI components discover variants by scanning the filesystem
- The Item Grid groups variants by base item name for clean display

### Design Principles

1. **Separation of Concerns**: Icon variants are filesystem artifacts, not data model properties
2. **Backward Compatibility**: Existing items without variants continue to work unchanged
3. **Convention over Configuration**: File naming convention eliminates need for variant metadata
4. **Minimal Data Model Changes**: No changes to Item struct or repository layer
5. **UI-Driven Discovery**: UI scans filesystem to discover variants dynamically

## Components and Interfaces

### 1. Icon Variant Discovery Service

**Location**: `internal/services/iconVariants.go`

**Purpose**: Provides filesystem operations for discovering, validating, and managing icon variant files.

**Interface**:
```go
type IconVariantService interface {
    // GetVariants returns all variant names for an item
    GetVariants(programName, itemName string) ([]string, error)
    
    // GetVariantPath returns the full path to a variant icon
    GetVariantPath(programName, itemName, variantName string) string
    
    // AddVariant copies a file to the icons directory with proper naming
    AddVariant(programName, itemName, variantName, sourcePath string) error
    
    // DeleteVariant removes a variant icon file
    DeleteVariant(programName, itemName, variantName string) error
    
    // GetBaseItemName extracts the base name from a full item name
    GetBaseItemName(fullItemName string) string
    
    // ValidateVariantFile checks if a file exists and is a valid PNG
    ValidateVariantFile(path string) error
}
```

**Implementation Details**:
- Uses `filepath.Glob()` to discover variant files matching pattern `{ItemName}|*.png`
- Parses filenames to extract variant names by splitting on ProgramDelimiter
- Validates PNG files using basic file header checks
- Returns sorted variant lists for consistent UI display

### 2. Item Grid Grouping Logic

**Location**: `ui/editor.go` (modifications to existing accordion population)

**Purpose**: Groups icon variants by base item name in the Item Gridwrap accordion item sections.

**Algorithm**:
1. Retrieve all items from ItemRepository
2. For each item, extract base name using `GetBaseItemName()`
3. Group items with same base name into a slice
4. Create one gridwrap item per base name
5. Sort gridwrap items alphabetically by base name

### 3. Icon Variant Editor Widget

**Location**: `ui/custom_widgets/iconVariantEditor.go`

**Purpose**: Custom Fyne widget for managing icon variants in the item editor panel.

**Structure**:
```go
type IconVariantEditor struct {
    widget.BaseWidget
    
    programName string
    itemName    string
    variants    []string
    service     *services.IconVariantService
    
    // UI components
    variantList   *fyne.Container
    addButton     *widget.Button
    onVariantChange func()
}
```

**Features**:
- Displays grid of icon thumbnails (64x64 pixels)
- Each thumbnail has variant name label and delete button
- "Add Icon Variant" button opens file picker
- Prompts for variant name on add
- Confirmation dialog on delete
- Prevents deletion of last variant
- Refreshes display after add/delete operations

### 4. Icon Thumbnail Widget

**Location**: `ui/custom_widgets/iconThumbnail.go`

**Purpose**: Displays a single icon variant with preview, label, and delete button.

**Structure**:
```go
type IconThumbnail struct {
    widget.BaseWidget
    
    iconPath    string
    variantName string
    onDelete    func()
    
    // UI components
    image       *canvas.Image
    label       *widget.Label
    deleteBtn   *widget.Button
}
```

**Features**:
- Loads PNG from filesystem
- Scales to 64x64 pixels maintaining aspect ratio
- Shows placeholder with error indicator if file missing/corrupted
- Delete button with danger styling
- Variant name label below thumbnail

## Data Models

### No Changes to Item Model

The existing `Item` struct remains unchanged:
```go
type Item struct {
    Name     string   `json:"name"`
    GridSize [2]int   `json:"gridSize"`
    Tags     []string `json:"tags"`
    StackMax int      `json:"stackMax"`
    Merchant string   `json:"merchant"`
}
```

**Rationale**: Icon variants are filesystem artifacts, not data properties. This maintains:
- Clean separation between data and presentation
- Backward compatibility with existing configurations
- Simplified serialization (no variant metadata in YAML)
- Single source of truth (filesystem is the source for variants)

### Filesystem Structure

```
internal/assets/images/icons/
├── dark and darker/
│   ├── Health Potion.png              # Legacy single icon (no variant)
│   ├── Mana Potion|Original.png       # Variant 1
│   ├── Mana Potion|Ice.png            # Variant 2
│   ├── Mana Potion|Bone.png           # Variant 3
│   └── Bandage|Red.png                # Single variant
└── path of exile 2/
    ├── Chaos Orb.png
    └── Exalted Orb|Shiny.png
```

## UI Integration

### Item Editor Panel Modifications

**Location**: `ui/editor.go` - `constructEditorTabs()` function

**Changes**:
1. Add new form item for icon variants section
2. Insert `IconVariantEditor` widget after existing form items
3. Wire up variant change callback to refresh accordion

**Updated Form Structure**:
```go
itw["iconVariants"] = NewIconVariantEditor(programName, itemName, iconService)
itw[form] = widget.NewForm(
    widget.NewFormItem(name, itw[name]),
    widget.NewFormItem(cols, itw[cols]),
    widget.NewFormItem(rows, itw[rows]),
    widget.NewFormItem(tags, widget.NewEntry()),
    widget.NewFormItem("", itw[tags]),
    widget.NewFormItem(sm, itw[sm]),
    widget.NewFormItem("Icon Variants", itw["iconVariants"]), // NEW
)
```

### Item Grid Accordion Modifications

**Current Behavior**: Shows all items including variants as separate entries

**New Behavior**: 
1. Group items by base name (text before "|")
2. Display one gridwrap item per base name
3. Show first variant's icon as gridwrap icon
4. When selected, load all variants in editor panel

**Implementation**:
- Modify gridwrap population logic to ensure duplicates item names are not added to the filtered list
- Update selection handler to load base item name
- IconVariantEditor discovers and displays all variants for selected item

## Error Handling

### File System Errors

**Scenario**: Icon file missing or corrupted
- **Detection**: `ValidateVariantFile()` checks file existence and PNG header
- **Handling**: Display placeholder image with error indicator in thumbnail
- **User Action**: User can delete invalid variant or replace file manually

**Scenario**: Insufficient permissions to write icon files
- **Detection**: `AddVariant()` returns error on file copy failure
- **Handling**: Show error dialog with permission issue message
- **User Action**: User must fix filesystem permissions

### Validation Errors

**Scenario**: User attempts to delete last variant
- **Detection**: `GetVariants()` returns count before delete operation
- **Handling**: Disable delete button when only one variant exists
- **User Action**: None - operation prevented

**Scenario**: Duplicate variant name
- **Detection**: Check existing variants before adding new one
- **Handling**: Show error dialog prompting for different name
- **User Action**: User enters unique variant name

**Scenario**: Invalid PNG file selected
- **Detection**: `ValidateVariantFile()` checks PNG header
- **Handling**: Show error dialog explaining file must be PNG
- **User Action**: User selects valid PNG file

## Testing Strategy

### Unit Tests

**IconVariantService Tests** (`internal/services/iconVariants_test.go`):
- `TestGetVariants`: Verify variant discovery from filesystem
- `TestGetBaseItemName`: Verify base name extraction with/without delimiter
- `TestAddVariant`: Verify file copy with proper naming
- `TestDeleteVariant`: Verify file deletion
- `TestValidateVariantFile`: Verify PNG validation logic

**Grouping Logic Tests** (`ui/editor_test.go`):
- `TestGroupItemsByBaseName`: Verify grouping with various item name patterns
- `TestGroupItemsByBaseName_NoVariants`: Verify backward compatibility
- `TestGroupItemsByBaseName_MixedVariants`: Verify mixed variant/non-variant items

### Integration Tests

**End-to-End Variant Management** (`internal/models/repositories/integration_test.go`):
- Create item, add variants, verify filesystem
- Delete variant, verify file removed
- Load item with variants, verify all discovered
- Test with multiple programs simultaneously

### Manual Testing Checklist

1. **Add Variant Flow**:
   - Select item in grid
   - Click "Add Icon Variant"
   - Select PNG file
   - Enter variant name
   - Verify thumbnail appears
   - Verify file exists in correct directory

2. **Delete Variant Flow**:
   - Select item with multiple variants
   - Click delete on one variant
   - Confirm deletion
   - Verify thumbnail removed
   - Verify file deleted from filesystem

3. **Grid Grouping**:
   - Create items with variants (e.g., "Potion|Red", "Potion|Blue")
   - Verify only one "Potion" entry in grid
   - Select "Potion" entry
   - Verify both variants shown in editor

4. **Backward Compatibility**:
   - Load existing program with non-variant items
   - Verify items display correctly
   - Add variant to existing item
   - Verify both old and new items work

5. **Error Scenarios**:
   - Try to delete last variant (should be prevented)
   - Select non-PNG file (should show error)
   - Enter duplicate variant name (should show error)
   - Delete icon file manually, reload UI (should show placeholder)

## Implementation Notes

### Constants Addition

Add to `internal/config/constants.go`:
```go
const (
    IconsPath = ImagesPath + "icons/"
    IconThumbnailSize = 64 // pixels
)
```

### Fyne Image Loading

Use Fyne's `canvas.NewImageFromFile()` for loading icon thumbnails:
```go
img := canvas.NewImageFromFile(iconPath)
img.FillMode = canvas.ImageFillContain
img.SetMinSize(fyne.NewSize(IconThumbnailSize, IconThumbnailSize))
```

### File Picker Configuration

Configure file picker to filter PNG files only:
```go
dialog.ShowFileOpen(func(reader fyne.URIReadCloser, err error) {
    // Handle file selection
}, window)
// Note: Fyne's file dialog doesn't support filters on all platforms
// Validate file type after selection
```

### Accordion Icon Display

Use first variant's icon for accordion item icon:
```go
variants, _ := iconService.GetVariants(programName, baseItemName)
if len(variants) > 0 {
    iconPath := iconService.GetVariantPath(programName, baseItemName, variants[0])
    icon := canvas.NewImageFromFile(iconPath)
    accordionItem.Icon = icon
}
```

## Migration Path

### Existing Items Without Variants

No migration needed. Items without "|" in filename continue to work as before.

### Converting Single Icons to Variants

Users can manually rename existing icon files to add variant suffix:
- Before: `Health Potion.png`
- After: `Health Potion|Original.png`

Or use the UI to add a new variant and delete the old file.

### Bulk Conversion Tool (Future Enhancement)

A CLI tool could be added to batch convert existing icons to variant format:
```bash
sqyre convert-icons --program "dark and darker" --variant "Original"
```

This would rename all non-variant icons to include the specified variant name.

## Performance Considerations

### Filesystem Scanning

- **Concern**: Scanning filesystem for variants on every UI refresh
- **Mitigation**: Cache variant lists per item, invalidate on add/delete
- **Expected Impact**: Negligible for typical item counts (<1000 items)

### Image Loading

- **Concern**: Loading many icon thumbnails simultaneously
- **Mitigation**: Lazy load thumbnails as they scroll into view
- **Expected Impact**: Minimal - 64x64 thumbnails are small (<10KB each)

### File Operations

- **Concern**: File copy/delete operations blocking UI
- **Mitigation**: Perform file operations in goroutines with progress feedback
- **Expected Impact**: Minimal - single file operations complete in <100ms

## Security Considerations

### Path Traversal

- **Risk**: User-provided variant names could contain path traversal characters
- **Mitigation**: Sanitize variant names to remove "../" and absolute paths
- **Implementation**: Use `filepath.Clean()` and validate no directory separators

### File Type Validation

- **Risk**: User could select non-PNG files causing display errors
- **Mitigation**: Validate PNG header before accepting file
- **Implementation**: Check first 8 bytes match PNG signature: `\x89PNG\r\n\x1a\n`

### Disk Space

- **Risk**: Users could add unlimited variants consuming disk space
- **Mitigation**: Set maximum of 100 variants per item (per requirements)
- **Implementation**: Check variant count before allowing add operation