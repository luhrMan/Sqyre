# Implementation Plan

- [ ] 1. Refactor LoadIconBytes() to use filesystem loading
  - [x] 1.1 Replace embed.FS calls with os package calls
    - Replace `iconFS.ReadDir()` with `os.ReadDir(config.IconsPath)`
    - Replace `iconFS.ReadFile()` with `os.ReadFile()`
    - Update path construction to use `filepath.Join()` with `config.IconsPath`
    - _Requirements: 1.1, 2.1, 2.2_

  - [x] 1.2 Implement graceful error handling
    - Add check for `os.IsNotExist()` to return empty map when icons directory is missing
    - Add error logging for unreadable program subdirectories (continue processing)
    - Add error logging for unreadable icon files (continue processing)
    - _Requirements: 2.3, 5.3_

  - [x] 1.3 Verify icon key format preservation
    - Ensure key construction uses `programName + config.ProgramDelimiter + filename`
    - Verify variant icons (with `|` in filename) are handled correctly
    - Test that non-variant icons maintain expected key format
    - _Requirements: 1.4, 3.4_

- [x] 2. Remove icon embed directives
  - [x] 2.1 Remove icon-specific embed code
    - Delete `//go:embed images/icons/*` directive
    - Delete `iconFS embed.FS` variable declaration
    - Verify `//go:embed images/icon.svg` and `appIcon` remain unchanged
    - _Requirements: 1.2, 1.3, 4.1, 4.2, 4.3_

  - [x] 2.2 Update package imports
    - Add `os` package import
    - Add `path/filepath` package import
    - Verify `embed` package remains for appIcon functionality
    - _Requirements: 4.4_

- [x] 3. Update and verify tests
  - [x] 3.1 Update existing test expectations
    - Verify `TestLoadIconBytes()` works with filesystem loading
    - Verify `TestBytesToFyneIcons()` continues to pass
    - Verify `TestGetIconBytes()` continues to pass
    - _Requirements: 5.1, 5.2_

  - [ ]* 3.2 Add test for missing directory scenario
    - Create test case that verifies empty map returned when icons directory doesn't exist
    - Verify no error is returned for missing directory
    - _Requirements: 5.3_

  - [ ]* 3.3 Add test for icon key format validation
    - Verify loaded icons follow `programName|filename` format
    - Test both variant and non-variant icon key formats
    - _Requirements: 5.4_

- [ ] 4. Manual verification and testing
  - [ ] 4.1 Build and run application
    - Execute `go build ./cmd/sqyre`
    - Launch application and verify icons display correctly in UI
    - Check logs for any icon loading errors
    - _Requirements: 1.1, 2.1_

  - [ ] 4.2 Test adding new program icons without rebuild
    - Create new subdirectory under `internal/assets/images/icons/`
    - Add PNG icon files to new directory
    - Restart application (without rebuild)
    - Verify new icons appear and are usable
    - _Requirements: 1.1, 2.1, 2.2_

  - [ ]* 4.3 Test icon variant functionality
    - Create icon file with variant naming (e.g., `ItemName|Variant1.png`)
    - Verify variant icon loads with correct key format
    - Test that variant icons work in UI
    - _Requirements: 1.4_
