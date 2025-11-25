# Requirements Document

## Introduction

This feature refactors the icon loading system in Squire to read icon files directly from the filesystem instead of using Go's embed functionality. The application icon (appIcon) will remain embedded for reliability, but all game/program-specific icons will be loaded dynamically from disk. This change eliminates the need to rebuild the application when users add new icons and provides more flexibility for multi-program support.

## Glossary

- **Icon System**: The subsystem responsible for loading and managing icon image resources used in the UI
- **Embed System**: Go's `embed` package functionality that compiles files into the binary at build time
- **Filesystem Loading**: Reading files from disk at runtime rather than from embedded resources
- **Icon Map**: The in-memory data structure (`map[string][]byte`) that stores loaded icon data
- **Program Delimiter**: The `|` character used to separate program name from item name in icon keys
- **Base Item Name**: The item name without variant suffix (e.g., "Health Potion" from "Health Potion|Variant1")
- **Icon Variants**: Multiple icon files for the same item with different visual representations

## Requirements

### Requirement 1

**User Story:** As a developer, I want to remove the embed system for game icons, so that users can add new icons without rebuilding the application

#### Acceptance Criteria

1. WHEN THE Icon System initializes, THE Icon System SHALL load icon files from the filesystem directory `internal/assets/images/icons/`
2. THE Icon System SHALL NOT use Go embed directives for game/program icon files
3. THE Icon System SHALL maintain the embedded appIcon for application branding
4. THE Icon System SHALL preserve the existing icon key format `programName|itemName.png` or `programName|itemName|variantName.png`

### Requirement 2

**User Story:** As a user, I want icons to load from the filesystem, so that I can add new program icons without technical knowledge

#### Acceptance Criteria

1. WHEN THE Icon System loads icons, THE Icon System SHALL read files from `internal/assets/images/icons/` subdirectories
2. WHEN a subdirectory exists under `internal/assets/images/icons/`, THE Icon System SHALL treat the subdirectory name as the program name
3. WHEN an icon file cannot be read, THE Icon System SHALL log the error and continue loading other icons
4. THE Icon System SHALL support PNG format icon files

### Requirement 3

**User Story:** As a developer, I want the icon loading API to remain unchanged, so that existing code continues to work without modifications

#### Acceptance Criteria

1. THE Icon System SHALL maintain the `LoadIconBytes()` function signature returning `(map[string][]byte, error)`
2. THE Icon System SHALL maintain the `GetIconBytes()` function signature returning `map[string][]byte`
3. THE Icon System SHALL maintain the `BytesToFyneIcons()` function signature returning `map[string]*fyne.StaticResource`
4. THE Icon System SHALL populate the icon map with the same key format as the current implementation

### Requirement 4

**User Story:** As a developer, I want to remove embed-related code, so that the codebase is simpler and easier to maintain

#### Acceptance Criteria

1. THE Icon System SHALL remove the `//go:embed images/icons/*` directive
2. THE Icon System SHALL remove the `iconFS embed.FS` variable
3. THE Icon System SHALL maintain the `//go:embed images/icon.svg` directive for appIcon only
4. THE Icon System SHALL maintain the `embed` package import only for appIcon functionality

### Requirement 5

**User Story:** As a developer, I want existing tests to pass with minimal changes, so that I can verify the refactor maintains functionality

#### Acceptance Criteria

1. WHEN tests execute, THE Icon System SHALL load icons from the filesystem in the test environment
2. THE Icon System SHALL maintain compatibility with existing test assertions in `embeds_test.go`
3. WHEN icon files are missing in test environment, THE Icon System SHALL return an empty map without errors
4. THE Icon System SHALL validate that loaded icons follow the program delimiter key format
