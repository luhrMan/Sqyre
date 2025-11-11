# Design Document

## Overview

This design document outlines the refactoring of the coordinates model to implement the repository pattern, aligning it with the existing ItemRepository architecture. The refactoring will move coordinate types from `internal/models/coordinates/` to `internal/models/`, create dedicated repository implementations for Points and SearchAreas, and integrate them into the Program model using lazy initialization and factory patterns.

## Architecture

### Current Architecture

```
internal/models/
├── coordinates/
│   ├── coordinates.go          # Coordinates, Point, SearchArea types (will be scoped to Program)
├── repositories/
│   ├── base.go                 # Generic BaseRepository
│   ├── item.go                 # ItemRepository (scoped to Program)
│   ├── macro.go                # MacroRepository
│   └── program.go              # ProgramRepository
└── program.go                  # Program model with Items and Coordinates
```

### Target Architecture

```
internal/models/
├── coordinates.go              # Point and SearchArea types (moved)
├── repositories/
│   ├── base.go                 # Generic BaseRepository
│   ├── coordinates.go          # PointRepository and SearchAreaRepository
│   ├── item.go                 # ItemRepository
│   ├── macro.go                # MacroRepository
│   └── program.go              # ProgramRepository
└── program.go                  # Program with lazy-initialized repos
```

## Components and Interfaces

### 1. Coordinate Models (`internal/models/coordinates.go`)

**Point**
```go
type Point struct {
    Name string
    X    int
    Y    int
}
```

**SearchArea**
```go
type SearchArea struct {
    Name    string
    LeftX   int
    TopY    int
    RightX  int
    BottomY int
}
```

**Coordinates Container** (remains in Program for serialization)
```go
type Coordinates struct {
    Points      map[string]*Point
    SearchAreas map[string]*SearchArea
}
```

### 2. Repository Interfaces (`internal/models/program.go`)

Define interfaces in the models package to avoid circular dependencies:

```go
type PointRepositoryInterface interface {
    Get(name string) (*Point, error)
    GetAll() map[string]*Point
    GetAllKeys() []string
    Set(name string, point *Point) error
    Delete(name string) error
    Save() error
    Count() int
}

type SearchAreaRepositoryInterface interface {
    Get(name string) (*SearchArea, error)
    GetAll() map[string]*SearchArea
    GetAllKeys() []string
    Set(name string, area *SearchArea) error
    Delete(name string) error
    Save() error
    Count() int
}
```

### 3. Repository Implementations (`internal/models/repositories/coordinates.go`)

**PointRepository**
```go
type PointRepository struct {
    mu            sync.RWMutex
    points        map[string]*models.Point  // Reference to Coordinates.Points
    resolutionKey string                    // e.g., "2560x1440"
    program       *models.Program           // Parent for saves
}

func NewPointRepository(program *models.Program, resolutionKey string) *PointRepository {
    coords := program.Coordinates[resolutionKey]
    if coords == nil {
        coords = &models.Coordinates{
            Points:      make(map[string]*models.Point),
            SearchAreas: make(map[string]*models.SearchArea),
        }
        program.Coordinates[resolutionKey] = coords
    }
    
    return &PointRepository{
        points:        coords.Points,
        resolutionKey: resolutionKey,
        program:       program,
    }
}
```

**SearchAreaRepository**
```go
type SearchAreaRepository struct {
    mu            sync.RWMutex
    searchAreas   map[string]*models.SearchArea  // Reference to Coordinates.SearchAreas
    resolutionKey string
    program       *models.Program
}

func NewSearchAreaRepository(program *models.Program, resolutionKey string) *SearchAreaRepository {
    coords := program.Coordinates[resolutionKey]
    if coords == nil {
        coords = &models.Coordinates{
            Points:      make(map[string]*models.Point),
            SearchAreas: make(map[string]*models.SearchArea),
        }
        program.Coordinates[resolutionKey] = coords
    }
    
    return &SearchAreaRepository{
        searchAreas:   coords.SearchAreas,
        resolutionKey: resolutionKey,
        program:       program,
    }
}
```

### 4. Program Model Integration (`internal/models/program.go`)

```go
type Program struct {
    Name        string
    Items       map[string]*Item
    Coordinates map[string]*Coordinates  // Keyed by resolution (e.g., "2560x1440")
    masks       map[string]func(f ...any) *gocv.Mat
    
    // Lazy-initialized repositories
    itemRepo       ItemRepositoryInterface
    pointRepos     map[string]PointRepositoryInterface      // Keyed by resolution
    searchAreaRepos map[string]SearchAreaRepositoryInterface // Keyed by resolution
    repoMu         sync.Mutex
}

// PointRepo returns a PointRepository for the given resolution
func (p *Program) PointRepo(resolutionKey string) PointRepositoryInterface {
    p.repoMu.Lock()
    defer p.repoMu.Unlock()
    
    if p.pointRepos == nil {
        p.pointRepos = make(map[string]PointRepositoryInterface)
    }
    
    if p.pointRepos[resolutionKey] == nil {
        if PointRepositoryFactory == nil {
            panic("PointRepositoryFactory not initialized")
        }
        p.pointRepos[resolutionKey] = PointRepositoryFactory(p, resolutionKey)
    }
    
    return p.pointRepos[resolutionKey]
}

// SearchAreaRepo returns a SearchAreaRepository for the given resolution
func (p *Program) SearchAreaRepo(resolutionKey string) SearchAreaRepositoryInterface {
    p.repoMu.Lock()
    defer p.repoMu.Unlock()
    
    if p.searchAreaRepos == nil {
        p.searchAreaRepos = make(map[string]SearchAreaRepositoryInterface)
    }
    
    if p.searchAreaRepos[resolutionKey] == nil {
        if SearchAreaRepositoryFactory == nil {
            panic("SearchAreaRepositoryFactory not initialized")
        }
        p.searchAreaRepos[resolutionKey] = SearchAreaRepositoryFactory(p, resolutionKey)
    }
    
    return p.searchAreaRepos[resolutionKey]
}
```

### 5. Factory Pattern (`internal/models/repositories/coordinates.go`)

```go
func init() {
    models.PointRepositoryFactory = func(p *models.Program, resKey string) models.PointRepositoryInterface {
        return NewPointRepository(p, resKey)
    }
    
    models.SearchAreaRepositoryFactory = func(p *models.Program, resKey string) models.SearchAreaRepositoryInterface {
        return NewSearchAreaRepository(p, resKey)
    }
}
```

## Data Models

### Coordinates Storage Structure

The Program model maintains a map of Coordinates keyed by resolution:

```yaml
programs:
  dark and darker:
    name: "dark and darker"
    items:
      health potion:
        name: "Health Potion"
        gridSize: [1, 2]
    coordinates:
      "2560x1440":
        points:
          stash-screen:
            name: "stash-screen"
            x: 1280
            y: 100
        searchAreas:
          stash-player-inv:
            name: "stash-player-inv"
            leftX: 100
            topY: 200
            rightX: 500
            bottomY: 600
```

### Resolution Key Format

Resolution keys follow the format: `"{width}x{height}"` (e.g., "2560x1440", "1920x1080")

This allows the same program to store coordinates for multiple screen resolutions.

## Error Handling

### Repository Errors

All repositories use standard error types from `internal/models/repositories/errors.go`:

- `ErrNotFound`: Coordinate does not exist
- `ErrInvalidKey`: Empty or invalid name
- `ErrSaveFailed`: Failed to persist to disk

### Error Wrapping

Errors are wrapped with context:
```go
return nil, fmt.Errorf("%w: point '%s' in program '%s' at resolution '%s'", 
    ErrNotFound, name, r.program.Name, r.resolutionKey)
```

### Panic Conditions

Factory functions panic if not initialized (similar to ItemRepository):
```go
if PointRepositoryFactory == nil {
    panic("PointRepositoryFactory not initialized - repositories package not imported")
}
```

## Testing Strategy

### Unit Tests

**PointRepository Tests** (`internal/models/repositories/coordinates_test.go`)
- Test CRUD operations (Get, Set, Delete)
- Test case-insensitive name lookups
- Test GetAll returns a copy
- Test Count and GetAllKeys
- Test error conditions (empty names, not found)

**SearchAreaRepository Tests**
- Same test coverage as PointRepository
- Verify SearchArea-specific fields

**Thread Safety Tests**
- Concurrent reads and writes
- Race condition detection using `go test -race`

**Persistence Tests**
- Verify Save() persists through ProgramRepository
- Verify changes are written to config.yaml
- Test multiple resolutions independently

### Integration Tests

**Program Integration** (`internal/models/repositories/integration_test.go`)
- Test lazy initialization of repositories
- Test multiple resolution keys
- Test interaction between PointRepo and SearchAreaRepo
- Verify backward compatibility with direct Coordinates map access

### Test Data

Use `testdata/` directory for test configuration files:
```
internal/models/repositories/testdata/
├── test_config.yaml
└── test_coordinates.yaml
```

## Migration Strategy

### Migration Path

1. **Phase 1**: Move types, create repositories (this spec)
2. **Phase 2**: Update internal code to use repositories
3. **Phase 3**: DELETE direct map access

### Import Updates

All files importing `internal/models/coordinates` must update to:
```go
import "Squire/internal/models"
```

Search pattern: `"Squire/internal/models/coordinates"`

## Design Decisions

### Why Separate Repositories for Points and SearchAreas?

- **Type Safety**: Each repository returns the correct type
- **Clear Intent**: Code explicitly states whether it's working with Points or SearchAreas
- **Independent Operations**: Different coordinate types may have different operations in the future

### Why Resolution-Keyed Repositories?

- **Multi-Resolution Support**: Users may switch between different screen resolutions
- **Isolation**: Changes to one resolution don't affect others
- **Flexibility**: Easy to add resolution-specific logic in the future

### Why Not Use BaseRepository?

BaseRepository is designed for top-level entities (Macros, Programs). Coordinates are nested within Programs and require:
- Parent Program reference for saves
- Resolution key scoping
- Reference to nested maps (not top-level)

### Why Keep Coordinates Struct?

- **Serialization**: Viper expects a struct for nested YAML
- **Atomicity**: Points and SearchAreas are logically grouped
- **Backward Compatibility**: Existing config files continue to work

## Performance Considerations

### Memory

- Repositories hold references to maps, not copies
- Lazy initialization prevents unnecessary repository creation
- GetAll() returns copies to prevent accidental modification

### Concurrency

- Read locks allow concurrent reads
- Write locks ensure exclusive access during modifications
- Mutex per repository instance (not global)

### Persistence

- Immediate persistence on Set/Delete (consistent with ItemRepository)
- Entire Program saved (not just coordinates) to maintain consistency
- Viper handles file I/O and YAML marshalling
