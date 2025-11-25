# Design Document: Map Key Synchronization

## Overview

This design addresses the data consistency issue where repository CRUD operations do not properly synchronize the map key with the model's internal key field. The solution involves modifying the `Set()` method in both `BaseRepository` and `NestedRepository` to automatically call `SetKey()` on the model before storing it, ensuring the model's internal state always matches the map key.

The fix is minimal, non-breaking, and leverages the existing `BaseModel` interface that all persistable models already implement.

## Architecture

### Current State

Both `BaseRepository` and `NestedRepository` currently implement the following pattern in their `Set()` method:

```go
func (r *Repository[T]) Set(key string, model *T) error {
    // 1. Validate inputs
    // 2. Normalize key to lowercase
    // 3. Store in map: r.models[normalizedKey] = model
    // 4. Persist to disk
}
```

**Problem**: The model's internal `Name` field (accessed via `GetKey()`) is never updated to match the normalized map key. This creates inconsistency.

### Proposed State

Add a single line to call `SetKey()` before storing:

```go
func (r *Repository[T]) Set(key string, model *T) error {
    // 1. Validate inputs
    // 2. Normalize key to lowercase
    // 3. Synchronize model key: model.SetKey(normalizedKey)  // NEW
    // 4. Store in map: r.models[normalizedKey] = model
    // 5. Persist to disk
}
```

This ensures the model's internal state always reflects the actual storage key.

## Components and Interfaces

### Affected Components

1. **BaseRepository** (`internal/models/repositories/base.go`)
   - Modify `Set()` method to call `SetKey()` before storing
   - No interface changes required

2. **NestedRepository** (`internal/models/repositories/nested.go`)
   - Modify `Set()` method to call `SetKey()` before storing
   - No interface changes required

### Type Constraint Challenge

**Issue**: The repositories use generic type parameter `T any`, but need to call `SetKey()` which is defined on the `BaseModel` interface.

**Current Signatures**:
```go
type BaseRepository[T any] struct { ... }
type NestedRepository[T any] struct { ... }
```

**Options**:

#### Option 1: Type Assertion (Recommended)
Keep `T any` and use type assertion to call `SetKey()`:

```go
func (r *BaseRepository[T]) Set(key string, model *T) error {
    // ... validation ...
    
    normalizedKey := strings.ToLower(key)
    
    // Synchronize model key if it implements BaseModel
    if baseModel, ok := any(model).(BaseModel); ok {
        baseModel.SetKey(normalizedKey)
    }
    
    r.models[normalizedKey] = model
    // ... persist ...
}
```

**Pros**:
- No breaking changes to existing code
- Works with all current model types
- Gracefully handles non-BaseModel types (though none exist)
- No changes to repository constructors or call sites

**Cons**:
- Runtime type assertion (minimal performance impact)
- Doesn't enforce BaseModel at compile time

#### Option 2: Generic Constraint
Change type parameter to require `BaseModel`:

```go
type BaseRepository[T BaseModel] struct { ... }

func (r *BaseRepository[T]) Set(key string, model *T) error {
    // ... validation ...
    normalizedKey := strings.ToLower(key)
    (*model).SetKey(normalizedKey)  // Compile-time safe
    r.models[normalizedKey] = model
    // ... persist ...
}
```

**Pros**:
- Compile-time type safety
- Explicit contract enforcement
- No runtime overhead

**Cons**:
- Breaking change: all repository instantiations must be updated
- Requires pointer receiver handling (`*T` implements `BaseModel`)
- More complex generic syntax

**Decision**: Use **Option 1 (Type Assertion)** because:
- All existing models already implement `BaseModel`
- No breaking changes required
- Simpler implementation
- Runtime cost is negligible (single interface check per Set operation)

## Data Models

No changes to data models required. All models already implement `BaseModel`:

- `Macro` - implements `GetKey()` and `SetKey()` via `Name` field
- `Program` - implements `GetKey()` and `SetKey()` via `Name` field  
- `Item` - implements `GetKey()` and `SetKey()` via `Name` field
- `Point` - implements `GetKey()` and `SetKey()` via `Name` field
- `SearchArea` - implements `GetKey()` and `SetKey()` via `Name` field

## Implementation Details

### BaseRepository.Set() Modification

**Location**: `internal/models/repositories/base.go`

**Current Code** (lines ~200-215):
```go
func (r *BaseRepository[T]) Set(key string, model *T) error {
	if key == "" {
		return ErrInvalidKey
	}
	if model == nil {
		return fmt.Errorf("model cannot be nil")
	}

	r.mu.Lock()
	normalizedKey := strings.ToLower(key)
	r.models[normalizedKey] = model
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist after set: %w", err)
	}

	return nil
}
```

**Modified Code**:
```go
func (r *BaseRepository[T]) Set(key string, model *T) error {
	if key == "" {
		return ErrInvalidKey
	}
	if model == nil {
		return fmt.Errorf("model cannot be nil")
	}

	r.mu.Lock()
	normalizedKey := strings.ToLower(key)
	
	// Synchronize model's internal key with the map key
	if baseModel, ok := any(model).(BaseModel); ok {
		baseModel.SetKey(normalizedKey)
	}
	
	r.models[normalizedKey] = model
	r.mu.Unlock()

	// Save immediately after modification
	if err := r.Save(); err != nil {
		return fmt.Errorf("failed to persist after set: %w", err)
	}

	return nil
}
```

### NestedRepository.Set() Modification

**Location**: `internal/models/repositories/nested.go`

**Current Code** (lines ~280-300):
```go
func (r *NestedRepository[T]) Set(key string, model *T) error {
	if key == "" {
		return ErrInvalidKey
	}
	if model == nil {
		return fmt.Errorf("model cannot be nil")
	}

	r.mu.Lock()
	normalizedKey := strings.ToLower(key)
	r.models[normalizedKey] = model
	r.mu.Unlock()

	// Save the parent aggregate immediately after modification
	if err := r.saveFunc(); err != nil {
		return fmt.Errorf("failed to persist aggregate after set (context: %s): %w", r.contextKey, err)
	}

	return nil
}
```

**Modified Code**:
```go
func (r *NestedRepository[T]) Set(key string, model *T) error {
	if key == "" {
		return ErrInvalidKey
	}
	if model == nil {
		return fmt.Errorf("model cannot be nil")
	}

	r.mu.Lock()
	normalizedKey := strings.ToLower(key)
	
	// Synchronize model's internal key with the map key
	if baseModel, ok := any(model).(BaseModel); ok {
		baseModel.SetKey(normalizedKey)
	}
	
	r.models[normalizedKey] = model
	r.mu.Unlock()

	// Save the parent aggregate immediately after modification
	if err := r.saveFunc(); err != nil {
		return fmt.Errorf("failed to persist aggregate after set (context: %s): %w", r.contextKey, err)
	}

	return nil
}
```

### Key Synchronization Logic

The synchronization logic is identical in both repositories:

```go
// Synchronize model's internal key with the map key
if baseModel, ok := any(model).(BaseModel); ok {
    baseModel.SetKey(normalizedKey)
}
```

**How it works**:
1. Convert `*T` to `any` (interface{})
2. Type assert to `BaseModel` interface
3. If successful, call `SetKey()` with the normalized key
4. If unsuccessful (model doesn't implement BaseModel), silently continue

**Why it's safe**:
- All current models implement `BaseModel`
- Type assertion is checked at runtime
- No panic if assertion fails
- Minimal performance overhead

## Error Handling

No new error conditions are introduced. The existing error handling remains:

- `ErrInvalidKey` - returned if key is empty string
- `fmt.Errorf("model cannot be nil")` - returned if model is nil
- Persistence errors - returned if Save() or saveFunc() fails

The type assertion for `SetKey()` is silent - if it fails, the model is stored without key synchronization. This maintains backward compatibility if any non-BaseModel types are used (though none currently exist).

## Testing Strategy

### Existing Tests

The fix should make existing tests pass without modification. Current tests already verify:

1. **BaseRepository Tests** (`internal/models/repositories/base_test.go`):
   - `TestBaseRepository_Set()` - verifies Set operation
   - `TestBaseRepository_Get()` - verifies retrieval
   - `TestBaseRepository_ConcurrentAccess()` - verifies thread safety

2. **MacroRepository Tests** (`internal/models/repositories/macro_test.go`):
   - Tests that use `Set()` with Macro models
   - Verifies persistence and retrieval

3. **ProgramRepository Tests** (`internal/models/repositories/program_test.go`):
   - Tests that use `Set()` with Program models
   - Verifies nested model operations

### New Test Cases

Add explicit key synchronization tests to verify the fix:

#### Test 1: Key Synchronization on Set
```go
func TestBaseRepository_KeySynchronization(t *testing.T) {
    repo := NewBaseRepository[testModel](...)
    
    model := &testModel{Name: "OldName", Value: 100}
    err := repo.Set("newname", model)
    
    // Verify model's internal key was updated
    assert.NoError(t, err)
    assert.Equal(t, "newname", model.GetKey())
    
    // Verify retrieval works with new key
    retrieved, err := repo.Get("newname")
    assert.NoError(t, err)
    assert.Equal(t, "newname", retrieved.GetKey())
}
```

#### Test 2: Case Normalization
```go
func TestBaseRepository_CaseNormalization(t *testing.T) {
    repo := NewBaseRepository[testModel](...)
    
    model := &testModel{Name: "Original", Value: 100}
    err := repo.Set("MixedCase", model)
    
    // Verify model key is normalized to lowercase
    assert.NoError(t, err)
    assert.Equal(t, "mixedcase", model.GetKey())
    
    // Verify case-insensitive retrieval
    retrieved, err := repo.Get("MIXEDCASE")
    assert.NoError(t, err)
    assert.Equal(t, "mixedcase", retrieved.GetKey())
}
```

#### Test 3: Rename Operation
```go
func TestBaseRepository_Rename(t *testing.T) {
    repo := NewBaseRepository[testModel](...)
    
    // Create model with original key
    model := &testModel{Name: "original", Value: 100}
    repo.Set("original", model)
    
    // Rename by setting with new key
    err := repo.Set("renamed", model)
    assert.NoError(t, err)
    
    // Verify old key is gone
    _, err = repo.Get("original")
    assert.Error(t, err)
    
    // Verify new key exists with updated model
    retrieved, err := repo.Get("renamed")
    assert.NoError(t, err)
    assert.Equal(t, "renamed", retrieved.GetKey())
    assert.Equal(t, 100, retrieved.Value)
}
```

#### Test 4: NestedRepository Key Synchronization
```go
func TestNestedRepository_KeySynchronization(t *testing.T) {
    program := models.NewProgram()
    itemRepo := NewItemRepository(program)
    
    item := &models.Item{Name: "old-name", GridSize: [2]int{1, 1}}
    err := itemRepo.Set("new-name", item)
    
    // Verify item's internal key was updated
    assert.NoError(t, err)
    assert.Equal(t, "new-name", item.GetKey())
    
    // Verify parent program was saved with correct key
    retrieved, err := itemRepo.Get("new-name")
    assert.NoError(t, err)
    assert.Equal(t, "new-name", retrieved.GetKey())
}
```

### Test Execution Plan

1. Run existing tests to establish baseline
2. Apply the fix to `BaseRepository.Set()` and `NestedRepository.Set()`
3. Run existing tests again - they should pass without modification
4. Add new key synchronization tests
5. Run full test suite to verify no regressions

### Manual Testing

Test with actual application usage:

1. **Macro Rename**: Create a macro, rename it via UI, verify config file shows correct key
2. **Program Rename**: Create a program, rename it, verify nested items still work
3. **Item Operations**: Add/rename items within a program, verify consistency
4. **Point/SearchArea**: Add/rename coordinates, verify they persist correctly

## Migration and Compatibility

### Backward Compatibility

✅ **Fully backward compatible** - no breaking changes:

- Repository interfaces unchanged
- Model interfaces unchanged
- All existing code continues to work
- No data migration required

### Forward Compatibility

✅ **Improved consistency** for future operations:

- New models automatically get key synchronization
- Rename operations work correctly
- Config files remain consistent

### Existing Data

No migration needed. Existing config files work as-is:

- Models loaded from disk have matching keys (they were saved correctly)
- Only runtime operations benefit from the fix
- Next save operation will ensure consistency

## Performance Considerations

### Runtime Overhead

Minimal performance impact:

- **Type assertion**: Single interface check per `Set()` operation (~1-2 ns)
- **SetKey() call**: Simple field assignment (~1 ns)
- **Total overhead**: < 5 ns per operation

### Memory Overhead

Zero additional memory:

- No new data structures
- No additional allocations
- Same memory footprint

### Concurrency

No impact on thread safety:

- `SetKey()` called within existing mutex lock
- No additional synchronization needed
- Same concurrency guarantees

## Alternatives Considered

### Alternative 1: UUID Map Keys

**Approach**: Use UUIDs for map keys, keep Name as separate field

**Rejected because**:
- Breaks human-readable config files
- Requires data migration
- Loses case-insensitive lookup benefits
- More complex UI integration
- Doesn't match existing architecture

### Alternative 2: Generic Constraint

**Approach**: Change `T any` to `T BaseModel` constraint

**Rejected because**:
- Breaking change to all repository instantiations
- More complex generic syntax
- Requires pointer receiver handling
- No significant benefit over type assertion

### Alternative 3: Separate Sync Method

**Approach**: Add `SyncKey()` method that callers must remember to call

**Rejected because**:
- Error-prone (easy to forget)
- Violates encapsulation
- Doesn't solve the root problem
- More complex API

### Alternative 4: Do Nothing

**Approach**: Document that callers must manually sync keys

**Rejected because**:
- Doesn't fix the bug
- Requires all callers to remember
- Inconsistent data is a real problem
- Simple fix is available

## Conclusion

The proposed solution is minimal, non-breaking, and leverages existing infrastructure. By adding a single type assertion and `SetKey()` call in both repository `Set()` methods, we ensure map keys and model keys remain synchronized without any breaking changes or performance impact.

The fix applies automatically to all five model types (Macro, Program, Item, Point, SearchArea) and prevents future inconsistency issues.
