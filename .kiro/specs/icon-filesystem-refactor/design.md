# Design Document: Icon Filesystem Refactor

## Overview

This design refactors the icon loading system in Squire to read game/program-specific icon files directly from the filesystem instead of using Go's embed functionality. The application icon (appIcon) will remain embedded for reliability and distribution simplicity, but all game icons will be loaded dynamically from disk at runtime.

### Goals

- Enable users to add new program icons without rebuilding the application
- Simplify the codebase by removing embed directives for game icons
- Maintain backward compatibility with existing icon loading API (no signature changes)
- Preserve the existing icon key format and data structures
- Support icon variants through filename-based convention

### Non-Goals

- Changing the icon key format or data structures
- Modifying how icons are used in the UI layer
- Adding icon hot-reloading or file watching capabilities
- Changing the appIcon embedding (it remains embedded)

## Architecture

### Current Architecture

```
embeds.go
├── //go:embed images/icon.svg → appIcon ([]byte)
├── //go:embed images/icons/* → iconFS (embed.FS)
└── LoadIconBytes() → reads from iconFS
    └── Walks directory structure
    └── Returns map[string][]byte with keys: "programName|filename.png"
```

### New Architecture

```
embeds.go
├── //go:embed images/icon.svg → appIcon ([]byte) [UNCHANGED]
├── LoadIconBytes() → reads from filesystem
    └── Uses filepath.Walk() or os.ReadDir()
    └── Reads from internal/assets/images/icons/
    └── Returns map[string][]byte with keys: "programName|filename.png"
```

### Key Changes

1. **Remove embed.FS for icons**: The `iconFS embed.FS` variable and `//go:embed images/icons/*` directive will be removed
2. **Filesystem reading**: Replace `iconFS.ReadDir()` and `iconFS.ReadFile()` with `os.ReadDir()` and `os.ReadFile()`
3. **Path resolution**: Use `filepath.Join()` with `config.IconsPath` constant for proper path construction
4. **Keep embed for appIcon**: The `//go:embed images/icon.svg` directive remains unchanged

## Components and Interfaces

### API Compatibility

**Critical Design Decision:** All public function signatures remain unchanged to satisfy Requirement 3 (API compatibility).

**Unchanged Functions:**
- `LoadIconBytes() (map[string][]byte, error)` - Signature preserved (Requirement 3.1)
- `GetIconBytes() map[string][]byte` - Signature preserved (Requirement 3.2)
- `BytesToFyneIcons() map[string]*fyne.StaticResource` - Signature preserved (Requirement 3.3)

**Rationale:** By maintaining identical function signatures, existing code throughout the application continues to work without modifications. Only the internal implementation of `LoadIconBytes()` changes from embed-based to filesystem-based loading.

### Modified Functions

#### `LoadIconBytes() (map[string][]byte, error)`

**Current Implementation:**
- Uses `iconFS.ReadDir("images/icons")` to list subdirectories
- Uses `iconFS.ReadFile()` to read icon bytes
- Embedded at compile time

**New Implementation:**
- Uses `os.ReadDir(config.IconsPath)` to list subdirectories
- Uses `os.ReadFile()` to read icon bytes from disk
- Loaded at runtime from `internal/assets/images/icons/`

**Algorithm:**
```
1. Read subdirectories from internal/assets/images/icons/
2. For each subdirectory (program name):
   a. Read all files in the subdirectory
   b. For each PNG file:
      - Read file bytes using os.ReadFile()
      - Store in map with key: "programName|filename.png"
      - Log errors but continue processing other files
3. Return populated icon map
```

**Error Handling:**
- If the icons directory doesn't exist, return empty map without error (graceful degradation per Requirement 3.3)
- If a specific icon file can't be read, log the error and continue (per Requirement 2.3)
- If a program subdirectory can't be read, log the error and continue processing other directories
- Only return error if the base directory read fails critically and cannot proceed

#### `GetIconBytes() map[string][]byte`

**No changes required** - Returns the package-level `icons` map

**Rationale:** Maintaining this function signature ensures backward compatibility (Requirement 3.2) and allows existing code to continue working without modifications.

#### `BytesToFyneIcons() map[string]*fyne.StaticResource`

**No changes required** - Converts byte map to Fyne resources

**Rationale:** This function operates on the icon map structure, which remains unchanged. Maintaining the signature satisfies Requirement 3.3 for API compatibility.

### Data Models

#### Icon Map Structure

```go
map[string][]byte
```

**Key Format:**
The Icon System uses the program delimiter (`|`) to construct icon keys. The key format automatically handles both variant and non-variant icons based on the filename:

- Non-variant icons: `"programName|ItemName.png"`
- Variant icons: `"programName|ItemName|VariantName.png"`

**Rationale:** By using the filename directly in the key construction (`programName + config.ProgramDelimiter + filename`), the system naturally supports icon variants without special handling. If a user creates a file named `ItemName|VariantName.png`, it becomes a variant automatically.

**Example Keys:**
- `"dark and darker|Health Potion.png"` (non-variant)
- `"dark and darker|Health Potion|Variant1.png"` (variant file)
- `"path of exile 2|Chaos Orb.png"` (non-variant)

### Directory Structure

```
internal/assets/images/icons/
├── dark and darker/
│   ├── Health Potion.png
│   ├── Health Potion|Variant1.png    # Variant icon (optional)
│   ├── Mana Potion.png
│   └── Gold Coin.png
├── path of exile 2/
│   ├── Chaos Orb.png
│   ├── Exalted Orb.png
│   └── Divine Orb.png
└── [other programs]/
    └── [icon files].png
```

**Note:** Icon variants are created by including the pipe delimiter (`|`) in the filename itself. The Icon System treats all PNG files equally and constructs keys using the full filename, which naturally supports the variant pattern.

## Implementation Details

### Path Resolution

Use the existing `config.IconsPath` constant which resolves to `"internal/assets/images/icons/"`:

```go
iconBasePath := config.IconsPath
entries, err := os.ReadDir(iconBasePath)
```

### File Reading Logic

```go
// Pseudocode for new implementation
func LoadIconBytes() (map[string][]byte, error) {
    icons := make(map[string][]byte)
    
    // Read program directories
    entries, err := os.ReadDir(config.IconsPath)
    if err != nil {
        // Graceful degradation if directory doesn't exist
        if os.IsNotExist(err) {
            return icons, nil
        }
        return nil, err
    }
    
    // Process each program directory
    for _, entry := range entries {
        if !entry.IsDir() {
            continue
        }
        
        programName := entry.Name()
        programPath := filepath.Join(config.IconsPath, programName)
        
        // Read icon files in program directory
        iconFiles, err := os.ReadDir(programPath)
        if err != nil {
            log.Printf("Could not read directory %s: %v", programPath, err)
            continue
        }
        
        // Load each icon file
        for _, iconFile := range iconFiles {
            if iconFile.IsDir() {
                continue
            }
            
            iconPath := filepath.Join(programPath, iconFile.Name())
            iconBytes, err := os.ReadFile(iconPath)
            if err != nil {
                log.Printf("Could not read icon %s: %v", iconPath, err)
                continue
            }
            
            // Store with program delimiter
            key := programName + config.ProgramDelimiter + iconFile.Name()
            icons[key] = iconBytes
        }
    }
    
    return icons, nil
}
```

### Import Changes

**Remove:**
- None - `embed` package must be retained for appIcon (per Requirement 4.3)

**Add:**
- `os` package (for file operations)
- `path/filepath` package (for path manipulation)

**Keep:**
- `embed` package (required for appIcon embedding per Requirement 4.4)

**Rationale:** The embed package cannot be removed entirely because the appIcon must remain embedded for reliability. Only the icon-specific embed directives are removed (per Requirements 4.1 and 4.2).

## Error Handling

### Error Scenarios

1. **Icons directory doesn't exist**
   - Behavior: Return empty map, no error (per Requirement 5.3)
   - Rationale: Graceful degradation for fresh installations and test environments

2. **Program subdirectory can't be read**
   - Behavior: Log error, skip directory, continue processing (per Requirement 2.3)
   - Rationale: One bad directory shouldn't break all icon loading; allows partial functionality

3. **Individual icon file can't be read**
   - Behavior: Log error, skip file, continue processing (per Requirement 2.3)
   - Rationale: Partial icon loading is better than complete failure; maximizes available icons

4. **Invalid file format (non-PNG)**
   - Behavior: Load anyway, let downstream code handle validation
   - Rationale: Flexibility for future format support; Requirement 2.4 specifies PNG but doesn't mandate rejection of other formats

### Logging Strategy

- Use `log.Printf()` for all errors (consistent with existing code)
- Include full file paths in error messages for debugging
- Log successful icon count at completion

## Testing Strategy

### Unit Tests

**Existing Tests (embeds_test.go):**
- `TestLoadIconBytes()` - Verify icon loading and key format
- `TestBytesToFyneIcons()` - Verify Fyne resource conversion
- `TestGetIconBytes()` - Verify icon retrieval

**Required Changes:**
- Tests will now load from filesystem instead of embedded resources (per Requirement 5.1)
- Test assertions remain the same to verify map structure and key format (per Requirement 5.2)
- Add test case for missing directory that verifies empty map is returned without errors (per Requirement 5.3)
- Verify that loaded icons follow the program delimiter key format (per Requirement 5.4)

**Rationale:** Minimal test changes ensure the refactor maintains existing functionality while validating the new filesystem-based loading mechanism.

### Test Data

The existing icon files in `internal/assets/images/icons/` will serve as test data:
- `dark and darker/` directory with ~150 icons
- `path of exile 2/` directory with ~30 icons

### Integration Testing

**Manual Verification:**
1. Run application and verify icons load correctly in UI
2. Add a new program directory with icons (without rebuild)
3. Verify new icons appear in the application
4. Remove icon files and verify graceful degradation

## Migration Path

### Step-by-Step Implementation

1. **Modify LoadIconBytes()** (addresses Requirements 1.1, 2.1, 2.2)
   - Replace `iconFS.ReadDir()` with `os.ReadDir()`
   - Replace `iconFS.ReadFile()` with `os.ReadFile()`
   - Update path construction to use `config.IconsPath`
   - Implement graceful error handling for missing directories

2. **Remove embed directives** (addresses Requirements 4.1, 4.2)
   - Remove `//go:embed images/icons/*` directive
   - Remove `iconFS embed.FS` variable
   - Keep `//go:embed images/icon.svg` for appIcon (per Requirement 1.3)

3. **Update imports** (addresses Requirement 4.4)
   - Add `os` and `path/filepath` packages
   - Keep `embed` package for appIcon functionality

4. **Run tests** (addresses Requirement 5)
   - Execute `go test ./internal/assets/...`
   - Verify all tests pass with filesystem loading
   - Confirm test compatibility with existing assertions

5. **Manual testing** (addresses Requirements 1.1, 2.1)
   - Build and run application
   - Verify icons display correctly
   - Test adding new program directory with icons without rebuild
   - Verify icon variants load correctly

### Rollback Plan

If issues arise, the embed system can be restored by:
1. Reverting changes to `embeds.go`
2. Restoring the `//go:embed` directive
3. Rebuilding the application

The icon files remain in place, so no data migration is needed.

## Performance Considerations

### Startup Time

**Current (Embedded):**
- Icons loaded from binary at compile time
- Near-instant access at runtime
- Binary size: ~5-10MB larger

**New (Filesystem):**
- Icons loaded from disk at startup
- Estimated load time: 50-200ms for ~200 icons
- Binary size: ~5-10MB smaller

**Impact:** Negligible startup delay (< 200ms) for significantly improved flexibility

### Memory Usage

- No change in runtime memory usage
- Icons still stored in `map[string][]byte` in memory
- Disk I/O only occurs once at startup

### Caching

- Icons loaded once at startup into package-level `icons` map
- No re-reading from disk during runtime
- Future enhancement: Add file watching for hot-reload

## Security Considerations

### Path Traversal

- Use `filepath.Join()` for all path construction
- Validate that resolved paths stay within `internal/assets/images/icons/`
- Don't accept user input for icon paths

### File Type Validation

- Current implementation accepts any file in icon directories
- Consider adding PNG header validation in future enhancement
- Malicious files would only affect UI rendering, not system security

## Dependencies

### New Dependencies

- None (uses standard library `os` and `path/filepath`)

### Removed Dependencies

- None (`embed` package may still be needed for appIcon)

## User Experience Impact

### For End Users (addresses Requirement 2)

**Before Refactor:**
- Adding new program icons requires developer knowledge
- Must modify source code and rebuild application
- High barrier to entry for customization

**After Refactor:**
- Users can add new program icons by creating a subdirectory under `internal/assets/images/icons/`
- No rebuild required - icons load at next application startup
- Simple file-based workflow: create folder, add PNG files, restart app

**Example Workflow:**
1. Navigate to `internal/assets/images/icons/`
2. Create new folder named after the program (e.g., "my game")
3. Add PNG icon files to the folder
4. Restart Squire - icons are now available

### For Developers (addresses Requirements 3 and 4)

**Benefits:**
- Simpler codebase without embed directives for icons (Requirement 4.1, 4.2)
- No API changes required in consuming code (Requirement 3)
- Easier debugging - can inspect icon files directly on disk
- Faster iteration during development (no rebuild to test new icons)

## Future Enhancements

1. **Icon Hot-Reload**: Watch filesystem for changes and reload icons dynamically
2. **Format Validation**: Validate PNG headers before loading
3. **Lazy Loading**: Load icons on-demand instead of at startup
4. **Icon Caching**: Cache processed Fyne resources to disk
5. **User Icon Directory**: Support loading icons from user-specified directories outside the application
6. **Icon Variant UI**: Add UI for managing icon variants per item

## Open Questions

None - design is complete and ready for implementation.
