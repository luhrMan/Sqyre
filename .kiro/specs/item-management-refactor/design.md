# Item Repository Design

## Overview

This design refactors item management in Squire to follow the established repository pattern while maintaining the aggregate relationship where Items belong to Programs. The ItemRepository provides a clean, consistent interface for item operations that mirrors MacroRepository and ProgramRepository, but operates within the context of a parent Program rather than as a standalone singleton.

## Architecture

### Aggregate Pattern

Items are **value objects** within the Program **aggregate root**. Unlike Macro and Program which are independent entities, Items have no lifecycle outside their parent Program.

```
┌─────────────────────────────────────────────┐
│           Application Layer                 │
│  - UI components, Services                  │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│         Repository Layer                    │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │  ProgramRepository (Singleton)       │  │
│  │  - Get/Set/Delete Programs           │  │
│  └──────────────────────────────────────┘  │
│                    ↓                        │
│  ┌──────────────────────────────────────┐  │
│  │  Program (Aggregate Root)            │  │
│  │  - Contains Items map                │  │
│  │  - Provides ItemsRepo() → ItemRepository │  │
│  └──────────────────────────────────────┘  │
│                    ↓                        │
│  ┌──────────────────────────────────────┐  │
│  │  ItemRepository (Instance per Program)│  │
│  │  - Get/Set/Delete Items              │  │
│  │  - Scoped to parent Program          │  │
│  └──────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│         Serialization Layer                 │
│  - Viper (Programs contain Items)           │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│         Storage Layer                       │
│  - config.yaml (nested structure)           │
└─────────────────────────────────────────────┘
```

### Key Design Principles

1. **Aggregate Boundaries**: Program is the aggregate root; Items are accessed through Program
2. **Consistency**: ItemRepository follows the same interface pattern as other repositories
3. **Scoped Access**: Each ItemRepository instance operates on one Program's items
4. **No Separate Persistence**: Items save through their parent Program
5. **Thread Safety**: ItemRepository protects concurrent access to the Program's Items map

## Components and Interfaces

### ItemRepository Structure

```go
// ItemRepository manages Item persistence within a Program context
type ItemRepository struct {
    mu          sync.RWMutex
    items       map[string]*models.Item  // Reference to Program.Items
    programName string                    // For logging and error context
    program     *models.Program           // Parent aggregate for saves
}
```

**Key Differences from BaseRepository:**
- Not a singleton (one instance per Program)
- Operates on a reference to Program.Items (not a separate map)
- Saves trigger Program-level persistence
- Includes programName for context in errors/logs

### ItemRepository Methods

```go
// Get retrieves an item by name (case-insensitive)
func (r *ItemRepository) Get(name string) (*models.Item, error)

// GetAll returns a copy of all items
func (r *ItemRepository) GetAll() map[string]*models.Item

// GetAllKeys returns sorted item names
func (r *ItemRepository) GetAllKeys() []string

// Set creates or updates an item
func (r *ItemRepository) Set(name string, item *models.Item) error

// Delete removes an item
func (r *ItemRepository) Delete(name string) error

// Save persists changes by saving the parent Program
func (r *ItemRepository) Save() error

// Count returns the number of items
func (r *ItemRepository) Count() int

// GetAllWithProgramPrefix returns items with "program|item" format
func (r *ItemRepository) GetAllWithProgramPrefix() map[string]*models.Item

// GetAllSorted returns alphabetically sorted item names
func (r *ItemRepository) GetAllSorted() []string
```

### Program Integration

```go
// In internal/models/program.go

type Program struct {
    Name        string
    Items       map[string]*Item
    Coordinates map[string]*coordinates.Coordinates
    masks       map[string]func(f ...any) *gocv.Mat
    
    itemRepo    *repositories.ItemRepository  // Lazy-initialized
    itemRepoMu  sync.Mutex                     // Protects itemRepo initialization
}

// Items returns an ItemRepository for managing this program's items
func (p *Program) ItemsRepo() *repositories.ItemRepository {
    p.itemRepoMu.Lock()
    defer p.itemRepoMu.Unlock()
    
    if p.itemRepo == nil {
        p.itemRepo = repositories.NewItemRepository(p)
    }
    return p.itemRepo
}
```

**Design Rationale:**
- Lazy initialization: ItemRepository created on first access
- Thread-safe initialization with separate mutex
- ItemRepository holds reference to Program.Items (no data duplication)
- Program remains the source of truth

### NewItemRepository Constructor

```go
// In internal/models/repositories/item.go

// NewItemRepository creates an ItemRepository for a Program
func NewItemRepository(program *models.Program) *ItemRepository {
    return &ItemRepository{
        items:       program.Items,  // Reference, not copy
        programName: program.Name,
        program:     program,
    }
}
```

## Data Models

### Item Model (No Changes)

```go
// In internal/models/program.go

type Item struct {
    Name     string   `json:"name"`
    GridSize [2]int   `json:"gridSize"`
    Tags     []string `json:"tags"`
    StackMax int      `json:"stackMax"`
    Merchant string   `json:"merchant"`
}
```

Items remain simple data structures with no persistence logic.

## Error Handling

### Error Usage

```go
// Get operation
item, err := program.ItemsRepo().Get("health potion")
if err != nil {
    if errors.Is(err, repositories.ErrNotFound) {
        // Handle missing item
    }
    return err
}

// Set operation
err := program.ItemsRepo().Set("health potion", &models.Item{
    Name:     "health potion",
    GridSize: [2]int{1, 1},
})
if err != nil {
    return fmt.Errorf("failed to save item: %w", err)
}
```

### Error Context

Errors include program name for better debugging:

```go
return fmt.Errorf("%w: item '%s' in program '%s'", 
    ErrNotFound, name, r.programName)
```

## Thread Safety

### Locking Strategy

```go
// Read operations (Get, GetAll, GetAllKeys)
func (r *ItemRepository) Get(name string) (*models.Item, error) {
    r.mu.RLock()
    defer r.mu.RUnlock()
    // ... access r.items
}

// Write operations (Set, Delete)
func (r *ItemRepository) Set(name string, item *models.Item) error {
    r.mu.Lock()
    defer r.mu.Unlock()
    // ... modify r.items
    
    // Save after modification (still holding lock)
    return r.Save()
}
```

**Key Points:**
- RWMutex allows concurrent reads
- Write operations hold exclusive lock through save
- Program.Items map is protected by ItemRepository's mutex
- No direct access to Program.Items from outside

## Testing Strategy

### Unit Tests

Test file: `internal/models/repositories/item_test.go`

**ItemRepository Tests:**
- CRUD operations with valid data
- Error handling for invalid operations (empty names, not found)
- Key normalization (case insensitivity)
- Thread safety with concurrent reads/writes
- GetAllWithProgramPrefix formatting
- GetAllSorted ordering

**Test Structure:**
```go
func TestItemRepository_Get(t *testing.T) {
    program := models.NewProgram()
    program.Name = "test game"
    program.Items["health potion"] = &models.Item{Name: "health potion"}
    
    repo := repositories.NewItemRepository(program)
    
    // Test successful get
    item, err := repo.Get("health potion")
    // assertions...
    
    // Test case insensitivity
    item, err = repo.Get("HEALTH POTION")
    // assertions...
    
    // Test not found
    _, err = repo.Get("nonexistent")
    // assert error is ErrNotFound
}
```

### Integration Tests

Test file: `internal/models/repositories/item_test.go` (integration tests included)

**Important**: Integration tests that verify persistence must use the `setupIntegrationTest()` helper function. This helper:
- Creates a temporary config file for testing
- Sets the `SQYRE_TEST_MODE` environment variable to enable proper reload behavior
- Configures Viper to use the temporary config
- Returns a cleanup function to restore state after the test

**Integration Test Pattern:**
```go
func TestItemRepository_Integration_Example(t *testing.T) {
    _, cleanup := setupIntegrationTest(t)
    defer cleanup()
    
    resetProgramRepo()
    
    // Create and save program
    program := models.NewProgram()
    program.Name = "Test Program"
    program.Items["item1"] = &models.Item{Name: "Item 1"}
    
    err := ProgramRepo().Set("testprogram", program)
    if err != nil {
        t.Fatalf("Failed to save: %v", err)
    }
    
    // Reload from disk
    err = ProgramRepo().Reload()
    if err != nil {
        t.Fatalf("Failed to reload: %v", err)
    }
    
    // Verify persistence
    reloaded, err := ProgramRepo().Get("testprogram")
    if err != nil {
        t.Fatalf("Failed to get reloaded program: %v", err)
    }
    
    // Verify items persisted
    if len(reloaded.Items) != 1 {
        t.Errorf("Expected 1 item, got %d", len(reloaded.Items))
    }
}
```

**Test Coverage:**
- Create Program with items and save through ProgramRepository
- Reload Program and verify items persisted correctly
- Modify items through ItemRepository and verify changes saved to disk
- Verify ItemRepository.Save() triggers Program save
- Verify ItemRepository.Delete() triggers Program save
- Test accessing ItemRepository through Program.ItemRepo()

## Migration Strategy

### Phase 1: Create ItemRepository

1. Create `internal/models/repositories/item.go`
2. Implement ItemRepository with all methods
3. Add unit tests
4. Add Program.ItemsRepo() method

### Phase 2: Migrate Internal Usage

1. Update Program methods to use ItemRepository internally:
   ```go
   // Old
   func (p *Program) GetItem(name string) (*Item, error) {
       if item, ok := p.Items[strings.ToLower(name)]; ok {
           return item, nil
       }
       return nil, fmt.Errorf("item not found")
   }
   
   // New (delegates to ItemRepository)
   func (p *Program) GetItem(name string) (*Item, error) {
       return p.ItemsRepo().Get(name)
   }
   ```

2. Keep old methods as wrappers during migration
3. Mark old methods as deprecated with comments

### Phase 3: Update Callers

1. Update UI components:
   ```go
   // Old
   item, err := program.GetItem("health potion")
   
   // New
   item, err := program.ItemsRepo().Get("health potion")
   ```

2. Update services similarly
3. Update one file at a time, test after each change

### Phase 4: Remove Old Methods

1. Once all callers updated, remove deprecated Program methods:
   - `GetItem()`
   - `SetItem()`
   - `NewItem()`
   - `AddItem()`
   - `DeleteItem()`

2. Keep helper methods that provide unique functionality:
   - `GetItemsWithAppendedProgramName()` → becomes `ItemsRepo().GetAllWithProgramPrefix()`
   - `GetItemsAsStringSlice()` → becomes `ItemsRepo().GetAllKeys()`
   - `SortItemsByName()` → becomes `ItemsRepo().GetAllSorted()`

## Design Decisions

### Why Not Use BaseRepository?

**Decision**: Create a specialized ItemRepository instead of using BaseRepository[Item].

**Rationale**:
- Items don't have independent lifecycle (no singleton needed)
- Items are scoped to a Program (need program context)
- Items save through parent Program (different persistence model)
- Items use a reference to Program.Items (not a separate map)
- Special methods like GetAllWithProgramPrefix are item-specific

**Trade-offs**: Some code duplication, but clearer semantics and simpler implementation.

### Why Reference Program.Items Instead of Copy?

**Decision**: ItemRepository holds a reference to Program.Items, not a copy.

**Rationale**:
- Single source of truth (no synchronization issues)
- Changes immediately visible to Program
- No memory overhead from duplication
- Simpler implementation

**Trade-offs**: ItemRepository lifetime tied to Program, but this matches the aggregate pattern.

### Why Lazy Initialize ItemRepository?

**Decision**: Create ItemRepository on first call to Program.ItemsRepo(), not in NewProgram().

**Rationale**:
- Not all code paths need ItemRepository
- Avoids circular dependency during Program construction
- Simpler Program initialization
- ItemRepository is lightweight (just holds references)

**Trade-offs**: Need mutex for thread-safe initialization, but this is minimal overhead.

### Why Keep GetAllWithProgramPrefix?

**Decision**: Maintain special formatting methods in ItemRepository.

**Rationale**:
- UI components need "program|item" format for display
- Delimiter pattern is established in codebase
- Encapsulates formatting logic in repository
- Makes migration easier (equivalent functionality)

**Trade-offs**: ItemRepository has more methods than BaseRepository, but they're all item-specific.

### How Does Save Work?

**Decision**: ItemRepository.Save() calls ProgramRepository.Set() to persist the parent Program.

**Implementation**:
```go
func (r *ItemRepository) Save() error {
    // Save the entire program (which includes all items)
    return repositories.ProgramRepo().Set(r.programName, r.program)
}
```

**Rationale**:
- Program is the unit of persistence
- Items don't have separate config file
- Maintains aggregate boundary
- Leverages existing ProgramRepository

**Trade-offs**: Saving one item saves entire Program, but Programs are small and this maintains consistency.

## Usage Examples

### Basic CRUD Operations

```go
// Get a program
program, err := repositories.ProgramRepo().Get("dark and darker")
if err != nil {
    return err
}

// Get an item
healthPotion, err := program.ItemsRepo().Get("health potion")
if err != nil {
    if errors.Is(err, repositories.ErrNotFound) {
        // Create new item
        healthPotion = &models.Item{
            Name:     "health potion",
            GridSize: [2]int{1, 1},
            Tags:     []string{"consumable", "healing"},
            StackMax: 5,
        }
        err = program.ItemsRepo().Set("health potion", healthPotion)
    }
    return err
}

// Update item
healthPotion.StackMax = 10
err = program.ItemsRepo().Set("health potion", healthPotion)

// Delete item
err = program.ItemsRepo().Delete("old item")

// Get all items
allItems := program.ItemsRepo().GetAll()

// Get sorted item names
itemNames := program.ItemsRepo().GetAllSorted()
```

### UI Integration

```go
// Populate dropdown with items
program, _ := repositories.ProgramRepo().Get(selectedProgram)
itemNames := program.ItemsRepo().GetAllSorted()
dropdown.SetOptions(itemNames)

// Display items with program prefix
itemsWithPrefix := program.ItemsRepo().GetAllWithProgramPrefix()
for displayName, item := range itemsWithPrefix {
    // displayName is "dark and darker|health potion"
    list.Append(displayName)
}
```

### Concurrent Access

```go
// Safe to call from multiple goroutines
var wg sync.WaitGroup
program, _ := repositories.ProgramRepo().Get("dark and darker")

// Multiple readers
for i := 0; i < 10; i++ {
    wg.Add(1)
    go func() {
        defer wg.Done()
        items := program.ItemsRepo().GetAll()
        // ... process items
    }()
}

// Single writer
wg.Add(1)
go func() {
    defer wg.Done()
    program.ItemsRepo().Set("new item", &models.Item{Name: "new item"})
}()

wg.Wait()
```
