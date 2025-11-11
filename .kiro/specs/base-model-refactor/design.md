# Design Document

## Overview

This design refactors the repository pattern to eliminate code duplication by introducing a proper BaseModel interface and leveraging Go generics. The current implementation has a `BaseModel` struct but doesn't fully utilize it - repositories like ItemRepository, PointRepository, and SearchAreaRepository duplicate CRUD logic instead of reusing the generic BaseRepository. This design will create a clean abstraction that works for both top-level models (Macro, Program) and nested models (Item, Point, SearchArea) while maintaining aggregate root relationships.

## Architecture

### Current State

```
BaseModel (struct with Name field)
    ↓
BaseRepository[T models.BaseModel] (generic, but underutilized)
    ↓
MacroRepository, ProgramRepository (use BaseRepository)

Separate implementations:
- ItemRepository (duplicates CRUD logic)
- PointRepository (duplicates CRUD logic)  
- SearchAreaRepository (duplicates CRUD logic)
```

### Target State

```
BaseModel (interface with GetKey/SetKey)
    ↓
BaseRepository[T BaseModel] (generic repository for all models)
    ↓
    ├── MacroRepository (top-level, uses BaseRepository)
    ├── ProgramRepository (top-level, uses BaseRepository)
    └── NestedRepository[T BaseModel] (adds aggregate root context)
            ↓
            ├── ItemRepository (nested in Program)
            ├── PointRepository (nested in Program/Resolution)
            └── SearchAreaRepository (nested in Program/Resolution)
```

## Components and Interfaces

### 1. BaseModel Interface

**Location:** `internal/models/base.go`

```go
// BaseModel defines the interface that all persistable models must implement.
// This interface enables generic repository operations by providing a standard
// way to access and modify the model's unique identifier.
//
// Example implementation:
//   type MyModel struct {
//       Name string
//       // other fields...
//   }
//
//   func (m *MyModel) GetKey() string { return m.Name }
//   func (m *MyModel) SetKey(key string) { m.Name = key }
type BaseModel interface {
    // GetKey returns the unique identifier for this model instance.
    // Keys are typically normalized to lowercase by repositories.
    GetKey() string
    
    // SetKey updates the unique identifier for this model instance.
    SetKey(key string)
}
```

**Design Rationale:**
- Interface instead of struct allows flexibility in how models store their key
- GetKey/SetKey pattern is idiomatic Go and works well with generics
- Keeps the interface minimal - only what's needed for repository operations

### 2. Model Implementations

Each model will implement BaseModel by providing GetKey/SetKey methods:

**Macro:**
```go
type Macro struct {
    Name        string
    Root        *actions.Loop
    GlobalDelay int
    Hotkey      []string
}

func (m *Macro) GetKey() string { return m.Name }
func (m *Macro) SetKey(key string) { m.Name = key }
```

**Program:**
```go
type Program struct {
    Name        string
    Items       map[string]*Item
    Coordinates map[string]*Coordinates
    // ... other fields
}

func (p *Program) GetKey() string { return p.Name }
func (p *Program) SetKey(key string) { p.Name = key }
```

**Item, Point, SearchArea:** Similar implementations using their Name fields.

### 3. BaseRepository Updates

**Location:** `internal/models/repositories/base.go`

**Changes:**
1. Update type constraint from `models.BaseModel` (struct) to `BaseModel` (interface)
2. Replace direct field access (`model.Name`) with interface methods (`model.GetKey()`, `model.SetKey()`)
3. No changes to method signatures or behavior

**Key Methods:**
- `Get(key string) (*T, error)` - Uses `model.GetKey()` for lookups
- `Set(key string, model *T) error` - Calls `model.SetKey(key)` before storing
- `GetAll()`, `GetAllKeys()`, `Delete()`, `Save()`, `Reload()`, `Count()` - Unchanged behavior

### 4. NestedRepository Pattern

**Location:** `internal/models/repositories/nested.go` (new file)

```go
// NestedRepository manages models that exist within an aggregate root context.
// Unlike BaseRepository which persists directly to config, NestedRepository
// saves changes by persisting the parent aggregate root.
//
// Example: Items are nested within a Program. When an Item changes, we save
// the entire Program (the aggregate root) rather than Items independently.
type NestedRepository[T BaseModel] struct {
    mu          sync.RWMutex
    models      map[string]*T
    contextKey  string                    // e.g., program name or "program|resolution"
    saveFunc    func() error              // Function to save the parent aggregate
}

// NewNestedRepository creates a repository for models within an aggregate root.
// models: reference to the parent's model map (e.g., program.Items)
// contextKey: identifier for logging/errors (e.g., "dark and darker")
// saveFunc: function that persists the parent aggregate (e.g., save Program)
func NewNestedRepository[T BaseModel](
    models map[string]*T,
    contextKey string,
    saveFunc func() error,
) *NestedRepository[T] {
    return &NestedRepository[T]{
        models:     models,
        contextKey: contextKey,
        saveFunc:   saveFunc,
    }
}

// Get, GetAll, GetAllKeys, Set, Delete, Count - similar to BaseRepository
// Save() calls saveFunc instead of Viper.WriteConfig()
```

**Design Rationale:**
- Maintains aggregate root pattern - child entities don't save independently
- Reuses CRUD logic while customizing persistence behavior
- Generic implementation works for Item, Point, and SearchArea
- saveFunc provides flexibility for different aggregate root types

### 5. Repository Refactoring

#### ItemRepository

**Before:** 200+ lines of duplicated CRUD code
**After:** Thin wrapper around NestedRepository

```go
type ItemRepository struct {
    *NestedRepository[models.Item]
    program *models.Program
}

func NewItemRepository(program *models.Program) *ItemRepository {
    return &ItemRepository{
        NestedRepository: NewNestedRepository[models.Item](
            program.Items,
            program.GetKey(),
            func() error {
                return ProgramRepo().Set(program.GetKey(), program)
            },
        ),
        program: program,
    }
}

// GetAllWithProgramPrefix - custom method specific to ItemRepository
func (r *ItemRepository) GetAllWithProgramPrefix() map[string]*models.Item {
    // Implementation using r.NestedRepository.GetAll()
}
```

#### PointRepository & SearchAreaRepository

Similar pattern - wrap NestedRepository with context-specific initialization:

```go
func NewPointRepository(program *models.Program, resKey string) *PointRepository {
    coords := program.Coordinates[resKey]
    if coords == nil {
        coords = &models.Coordinates{
            Points:      make(map[string]*models.Point),
            SearchAreas: make(map[string]*models.SearchArea),
        }
        program.Coordinates[resKey] = coords
    }
    
    return &PointRepository{
        NestedRepository: NewNestedRepository[models.Point](
            coords.Points,
            program.GetKey() + "|" + resKey,
            func() error {
                return ProgramRepo().Set(program.GetKey(), program)
            },
        ),
        resolutionKey: resKey,
        program:       program,
    }
}
```

## Data Models

No changes to data structures - only adding interface methods:

```go
// Before
type Macro struct {
    Name string
    // ...
}

// After
type Macro struct {
    Name string
    // ...
}

func (m *Macro) GetKey() string { return m.Name }
func (m *Macro) SetKey(key string) { m.Name = key }
```

## Error Handling

Maintain existing error types and behavior:
- `ErrNotFound` - Model doesn't exist
- `ErrInvalidKey` - Empty key provided
- `ErrSaveFailed` - Persistence failed
- `ErrLoadFailed` - Reload failed

Error messages should include context (model type, key, program name, resolution) for debugging.

## Testing Strategy

### Unit Tests

1. **BaseModel Interface Tests** (`internal/models/base_test.go`)
   - Verify each model implements BaseModel correctly
   - Test GetKey/SetKey behavior

2. **BaseRepository Tests** (`internal/models/repositories/base_test.go`)
   - Existing tests should pass without modification
   - Verify interface methods work correctly

3. **NestedRepository Tests** (`internal/models/repositories/nested_test.go`)
   - Test CRUD operations
   - Verify saveFunc is called on Set/Delete
   - Test thread safety

4. **Repository-Specific Tests**
   - Existing tests for Macro, Program, Item, Point, SearchArea repositories
   - Should pass without modification after refactor

### Integration Tests

1. **Cross-Repository Tests** (`internal/models/repositories/integration_test.go`)
   - Test Program → Item relationship
   - Test Program → Point/SearchArea relationships
   - Verify aggregate root persistence

### Test Approach

- Run existing tests first to establish baseline
- Refactor incrementally, running tests after each change
- Add new tests for NestedRepository pattern
- Verify no behavioral changes in existing functionality

## Migration Path

1. **Phase 1:** Create BaseModel interface, add methods to all models
2. **Phase 2:** Update BaseRepository to use interface instead of struct
3. **Phase 3:** Create NestedRepository pattern
4. **Phase 4:** Refactor ItemRepository to use NestedRepository
5. **Phase 5:** Refactor PointRepository to use NestedRepository
6. **Phase 6:** Refactor SearchAreaRepository to use NestedRepository
7. **Phase 7:** Run all tests and verify behavior
8. **Phase 8:** Update documentation and comments

Each phase maintains backward compatibility and passes all tests before proceeding.
