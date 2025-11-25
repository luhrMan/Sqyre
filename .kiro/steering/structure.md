---
inclusion: always
---

# Project Structure

## Directory Organization

```
Squire/
├── cmd/sqyre/              # Application entry point
├── internal/               # Private application code
│   ├── archive/           # Data binding utilities for tree widgets
│   ├── assets/            # Embedded resources (images, icons, SVGs, theme)
│   ├── config/            # Configuration constants and data files
│   ├── models/            # Core domain models
│   │   ├── actions/       # Action types (Click, Move, Key, Wait, Loop, etc.)
│   │   ├── repositories/  # Data persistence layer
│   │   └── serialize/     # Gopkg Yaml v3-based serialization
│   └── services/          # Business logic services
├── ui/                    # Fyne GUI components
│   └── custom_widgets/    # Custom Fyne widgets
├── binders/               # UI data binding setup
└── fyne-cross/            # Cross-compilation artifacts
```

## Architecture Patterns

### Action System

Actions follow a tree-based hierarchy with two main categories:

- **Leaf Actions**: Simple operations (Click, Move, Key, Wait)
- **Advanced Actions**: Container actions with sub-actions (Loop, ImageSearch, OCR, Conditional)

All actions implement `ActionInterface`. Advanced actions also implement `AdvancedActionInterface` with `GetSubActions()` and `SetSubActions()`.

Each action has:
- `Type` field for identification
- `uid` (UUID) for unique identification
- `Parent` reference for tree navigation
- `String()` method for display

### Repository Pattern

Models use repository pattern for persistence:
- `repositories.MacroRepo()` - Manages macro definitions
- `repositories.ProgramRepo()` - Manages program-specific configurations

### Service Layer

Services in `internal/services/`:
- `executor.go` - Executes action trees recursively
- `imageSearch.go` - Computer vision template matching
- `ocr.go` - Text recognition
- `hotkeys.go` - Global hotkey management
- `progressbar.go` - UI progress tracking

### UI Architecture

- `ui/ui.go` - Main UI initialization and construction
- `ui/macro*.go` - Macro management UI components
- `ui/actiontabs.go` - Action editor tabs
- `ui/editor.go` - Action property editors
- `binders/` - Connects UI widgets to data models using Fyne data binding

## Naming Conventions

- **Constants**: Use descriptive names with context (e.g., `StashScrPlayerInv`, `MerchantsScrStashInv`)
- **Delimiter**: Use `|` to separate program name from property (e.g., `"dark and darker|Health potion"`)
- **File extensions**: Defined as constants (`PNG`, `YAML`, `JSON`, `GOB`)
- **Paths**: Relative paths defined as constants in `internal/config/constants.go`

## Game-Specific Assets

Assets organized by game under `internal/assets/images/`:
- `icons/*program*/` - Item icons for any program the user adds
- `calibration/` - UI corner detection images

## Key Files

- `cmd/sqyre/sqyre.go` - Application bootstrap, initialization order, system tray setup
- `internal/models/macro.go` - Macro model with Root Loop and hotkey binding
- `internal/models/actions/interfaces.go` - Action interface definitions
- `internal/services/executor.go` - Core execution engine with recursive action processing
- `internal/config/constants.go` - Centralized constant definitions
