# Repository Pattern Design

## Overview

This design establishes a clean, standardized repository pattern for Squire's data persistence layer. The pattern separates domain models from persistence logic, provides thread-safe operations, and uses Go generics for type safety. The implementation builds on the existing Viper-based YAML serialization while improving code organization and error handling.

## Architecture

### Layered Architecture

```
┌─────────────────────────────────────┐
│     Application Layer (UI/Services) │
│  - Uses repositories for data access│
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│      Repository Layer               │
│  - MacroRepository                  │
│  - ProgramRepository                │
│  - Generic Repository[T]            │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│      Serialization Layer            │
│  - Viper configuration management   │
│  - Custom decode hooks              │
└─────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────┐
│      Storage Layer                  │
│  - config.yaml file                 │
└─────────────────────────────────────┘
```

### Key Design Principles

1. **Single Responsibility**: Repositories handle only data access; models handle only business logic
2. **Type Safety**: Use Go generics to ensure compile-time type checking
3. **Thread Safety**: Protect shared state with read-write mutexes
4. **Singleton Pattern**: One repository instance per model type
5. **Fail-Fast**: Return errors immediately rather than creating invalid state

## Components and Interfaces

### Core Repository Interface

```go
// Repository defines the standard interface for data access
type Repository[T any] interface {
    Get(key string) (*T, error)
    GetAll() map[string]*T
    GetAllKeys() []string
    Set(key string, model *T) error
    Delete(key string) error
    Save() error
    Reload() error
    Count() int
}
```

### Generic Repository Implementation

```go
// BaseRepository provides generic repository functionality
type BaseRepository[T any] struct {
    mu          sync.RWMutex
    models      map[string]*T
    configKey   string
    decodeFunc  func(key string) (*T, error)
    newFunc     func() *T
}
```

**Responsibilities:**
- Thread-safe CRUD operations
- Generic encode/decode orchestration
- Key normalization (lowercase)
- Error handling and logging

### Model-Specific Repositories

#### MacroRepository

```go
type MacroRepository struct {
    *BaseRepository[models.Macro]
}

var (
    macroRepo *MacroRepository
    macroOnce sync.Once
)

func MacroRepo() *MacroRepository {
    macroOnce.Do(func() {
        macroRepo = &MacroRepository{
            BaseRepository: NewBaseRepository[models.Macro](
                "macros",
                decodeMacro,
                models.NewMacro,
            ),
        }
        macroRepo.Reload()
    })
    return macroRepo
}
```

**Responsibilities:**
- Macro-specific decode logic
- Singleton initialization
- Macro constructor integration

#### ProgramRepository

```go
type ProgramRepository struct {
    *BaseRepository[models.Program]
}

var (
    programRepo *ProgramRepository
    programOnce sync.Once
)

func ProgramRepo() *ProgramRepository {
    programOnce.Do(func() {
        programRepo = &ProgramRepository{
            BaseRepository: NewBaseRepository[models.Program](
                "programs",
                decodeProgram,
                models.NewProgram,
            ),
        }
        programRepo.Reload()
    })
    return programRepo
}
```

**Responsibilities:**
- Program-specific decode logic
- Singleton initialization
- Program constructor integration

## Data Models

### Model Constructors

Models provide constructor functions but no persistence methods:

```go
// In internal/models/macro.go
func NewMacro(name string, delay int, hotkey []string) *Macro {
    return &Macro{
        Name:        name,
        Root:        actions.NewLoop(1, "root", []actions.ActionInterface{}),
        GlobalDelay: delay,
        Hotkey:      hotkey,
    }
}

// In internal/models/program.go
func NewProgram() *Program {
    return &Program{
        Items:       make(map[string]*Item),
        Coordinates: map[string]*coordinates.Coordinates{
            // ... initialization
        },
        masks: make(map[string]func(f ...any) *gocv.Mat),
    }
}
```

### Decode Functions

Decode functions live in the repository package and handle Viper unmarshalling:

```go
// decodeMacro unmarshals a macro from Viper configuration
func decodeMacro(key string) (*models.Macro, error) {
    keyPath := "macros." + key
    macro := &models.Macro{}
    
    err := serialize.GetViper().UnmarshalKey(
        keyPath,
        macro,
        viper.DecodeHook(serialize.MacroDecodeHookFunc()),
    )
    if err != nil {
        return nil, fmt.Errorf("failed to decode macro %s: %w", key, err)
    }
    
    return macro, nil
}

// decodeProgram unmarshals a program from Viper configuration
func decodeProgram(key string) (*models.Program, error) {
    keyPath := "programs." + key
    program := models.NewProgram()
    
    if err := serialize.GetViper().UnmarshalKey(keyPath+".name", &program.Name); err != nil {
        return nil, fmt.Errorf("failed to decode program name: %w", err)
    }
    
    if err := serialize.GetViper().UnmarshalKey(keyPath+".items", &program.Items); err != nil {
        return nil, fmt.Errorf("failed to decode program items: %w", err)
    }
    
    if err := serialize.GetViper().UnmarshalKey(keyPath+".coordinates", &program.Coordinates); err != nil {
        return nil, fmt.Errorf("failed to decode program coordinates: %w", err)
    }
    
    return program, nil
}
```

## Error Handling

### Error Types

```go
var (
    ErrNotFound      = errors.New("model not found")
    ErrInvalidKey    = errors.New("invalid key: cannot be empty")
    ErrSaveFailed    = errors.New("failed to save to disk")
    ErrLoadFailed    = errors.New("failed to load from disk")
    ErrDecodeFailed  = errors.New("failed to decode model")
)
```

### Error Handling Strategy

1. **Get Operations**: Return `ErrNotFound` if key doesn't exist
2. **Set Operations**: Validate input, return error if save fails
3. **Delete Operations**: No error if key doesn't exist (idempotent)
4. **Load Operations**: Log individual decode failures, continue loading others
5. **Save Operations**: Return error immediately if write fails

### Logging Strategy

- **Info Level**: Successful operations with counts (e.g., "Loaded 5 macros")
- **Warn Level**: Individual decode failures during bulk load
- **Error Level**: Critical failures that prevent operation completion

## Testing Strategy

### Unit Tests

Test files: `internal/models/repositories/*_test.go`

**BaseRepository Tests:**
- Thread safety with concurrent reads/writes
- CRUD operations with valid data
- Error handling for invalid operations
- Key normalization (case insensitivity)

**MacroRepository Tests:**
- Macro-specific decode logic
- Singleton initialization
- Integration with Viper decode hooks

**ProgramRepository Tests:**
- Program-specific decode logic
- Singleton initialization
- Complex nested structure decoding

### Integration Tests

Test file: `internal/models/repositories/integration_test.go`

- Load from actual config.yaml
- Save and reload cycle
- Multiple repository instances
- Concurrent access patterns

### Test Data

Create `testdata/config.yaml` with sample macros and programs for testing.

## Migration Strategy

### Phase 1: Create New Repository Structure

1. Create `internal/models/repositories/base.go` with generic repository
2. Create `internal/models/repositories/decode.go` with decode functions
3. Create `internal/models/repositories/errors.go` with error definitions

### Phase 2: Refactor Existing Repositories

1. Update `macro.go` to use BaseRepository
2. Update `program.go` to use BaseRepository
3. Remove old `repository.go` file

### Phase 3: Clean Up Models

1. Remove `Decode()` methods from `models.Macro`
2. Remove `Decode()` methods from `models.Program`
3. Remove commented-out encode/decode functions

### Phase 4: Update Callers

1. Update application initialization in `cmd/sqyre/sqyre.go`
2. Update UI components that access repositories
3. Update services that access repositories
4. Handle new error returns appropriately

## Design Decisions

### Why Generic BaseRepository?

**Decision**: Use Go generics for the base repository implementation.

**Rationale**: 
- Eliminates code duplication between MacroRepository and ProgramRepository
- Provides compile-time type safety
- Makes adding new repositories trivial
- Reduces maintenance burden

**Trade-offs**: Requires Go 1.18+, but project already uses Go 1.23.

### Why Separate Decode Functions?

**Decision**: Move decode logic from models to repository package.

**Rationale**:
- Models should not know about persistence mechanisms
- Easier to test models without Viper dependency
- Decode logic is data access concern, not business logic
- Allows swapping persistence layer without changing models

**Trade-offs**: Slightly more files, but clearer separation of concerns.

### Why RWMutex Instead of Mutex?

**Decision**: Use `sync.RWMutex` for repository state protection.

**Rationale**:
- Read operations (Get, GetAll) are more frequent than writes
- Multiple concurrent reads are safe and performant
- Write operations (Set, Delete, Save) need exclusive access
- Better performance under typical read-heavy workload

**Trade-offs**: Slightly more complex locking logic, but worth the performance gain.

### Why Not Return Empty Models on Get?

**Decision**: Return error when model not found instead of creating empty instance.

**Rationale**:
- Explicit error handling prevents bugs from silent failures
- Caller can decide whether to create new instance or handle error
- Matches Go idioms (e.g., map access returns ok boolean)
- Current auto-creation behavior hides missing data issues

**Trade-offs**: Requires updating callers to handle errors, but improves correctness.

### Why Keep Viper?

**Decision**: Continue using Viper for serialization rather than switching to encoding/json or encoding/gob.

**Rationale**:
- Already integrated and working
- Supports YAML which is human-readable for config files
- Custom decode hooks handle complex action tree structures
- Migration cost outweighs benefits

**Trade-offs**: Viper is heavyweight, but changing would require significant refactoring.

### How to Access Nested Collections (Items, Coordinates)?

**Decision**: Access nested collections through the parent aggregate (Program), not through separate repositories.

**Rationale**:
- Items and Coordinates are **value objects** that belong to a Program aggregate
- Program is the **aggregate root** - it maintains consistency of its internal collections
- Creating separate ItemRepository would violate aggregate boundaries
- Items don't have independent lifecycle - they're created/deleted with their Program
- Simpler API: `programRepo.Get("dark and darker").Items["health potion"]`

**Usage Pattern**:
```go
// Get a program, then access its items
program, err := repositories.ProgramRepo().Get("dark and darker")
if err != nil {
    return err
}

// Access items through the program
healthPotion := program.Items["health potion"]

// Modify items and save the entire program
program.Items["new item"] = &models.Item{Name: "new item", ...}
repositories.ProgramRepo().Set("dark and darker", program)
```

**When to Create Separate Repositories**:
- Only for **aggregate roots** that have independent lifecycle
- Examples: Macro, Program (both are top-level entities)
- Counter-examples: Item, Coordinates, Actions (all belong to parent aggregates)

**Trade-offs**: 
- Must load entire Program to access Items (acceptable - Programs are small)
- Saving one Item requires saving entire Program (acceptable - maintains consistency)
- Cannot query across all Items in all Programs easily (not a current requirement)
