# Implementation Plan

- [x] 1. Create BaseModel interface and update model implementations
  - [x] 1.1 Define BaseModel interface in `internal/models/base.go`
    - Replace the existing BaseModel struct with an interface
    - Add GetKey() string and SetKey(string) methods to the interface
    - Include comprehensive documentation with usage examples
    - _Requirements: 1.1_

  - [x] 1.2 Implement BaseModel interface for Macro
    - Add GetKey() method returning m.Name
    - Add SetKey(key string) method setting m.Name
    - Remove BaseModel struct embedding if present
    - _Requirements: 1.2_

  - [x] 1.3 Implement BaseModel interface for Program
    - Add GetKey() method returning p.Name
    - Add SetKey(key string) method setting p.Name
    - Remove BaseModel struct embedding if present
    - _Requirements: 1.3_

  - [x] 1.4 Implement BaseModel interface for Item
    - Add GetKey() method returning i.Name
    - Add SetKey(key string) method setting i.Name
    - Remove BaseModel struct embedding if present
    - _Requirements: 1.4_

  - [x] 1.5 Implement BaseModel interface for Point
    - Add GetKey() method returning p.Name
    - Add SetKey(key string) method setting p.Name
    - Remove BaseModel struct embedding if present
    - _Requirements: 1.5_

  - [x] 1.6 Implement BaseModel interface for SearchArea
    - Add GetKey() method returning sa.Name
    - Add SetKey(key string) method setting sa.Name
    - Ensure Name field exists in SearchArea struct
    - _Requirements: 1.6_

- [x] 2. Update BaseRepository to use BaseModel interface
  - [x] 2.1 Update BaseRepository type constraint
    - Change constraint from `models.BaseModel` struct to `BaseModel` interface
    - Update all method implementations to use GetKey() and SetKey()
    - Replace direct Name field access with interface method calls
    - _Requirements: 2.1, 2.3_

  - [x] 2.2 Verify BaseRepository method implementations
    - Ensure Get() uses model.GetKey() for lookups
    - Ensure Set() calls model.SetKey(key) before storing
    - Verify GetAll(), GetAllKeys(), Delete() work with interface
    - Confirm Save(), Reload(), Count() remain unchanged
    - _Requirements: 2.2, 2.4_

  - [x] 2.3 Update MacroRepository and ProgramRepository
    - Verify MacroRepository works with updated BaseRepository
    - Verify ProgramRepository works with updated BaseRepository
    - Ensure singleton initialization still functions correctly
    - Test backward compatibility with existing code
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [x] 3. Create NestedRepository pattern
  - [x] 3.1 Implement NestedRepository generic type
    - Create `internal/models/repositories/nested.go`
    - Define NestedRepository[T BaseModel] struct with mu, models, contextKey, saveFunc
    - Implement NewNestedRepository constructor
    - Add comprehensive documentation explaining aggregate root pattern
    - _Requirements: 4.1_

  - [x] 3.2 Implement NestedRepository CRUD operations
    - Implement Get(key string) (*T, error) with case-insensitive lookup
    - Implement GetAll() map[string]*T returning a copy
    - Implement GetAllKeys() []string returning sorted keys
    - Implement Set(key string, model *T) error calling saveFunc
    - Implement Delete(key string) error calling saveFunc
    - Implement Count() int
    - Ensure thread-safety with sync.RWMutex
    - _Requirements: 4.2, 5.3, 5.5_

- [x] 4. Refactor ItemRepository to use NestedRepository
  - [x] 4.1 Update ItemRepository structure
    - Embed NestedRepository[models.Item] instead of custom implementation
    - Keep program reference for custom methods
    - Update NewItemRepository to use NewNestedRepository
    - Pass program.Items map reference to NestedRepository
    - _Requirements: 4.3_

  - [x] 4.2 Remove duplicated CRUD code from ItemRepository
    - Delete Get, GetAll, GetAllKeys, Set, Delete, Count implementations
    - Keep GetAllWithProgramPrefix and GetAllSorted as custom methods
    - Ensure custom methods use embedded NestedRepository methods
    - _Requirements: 4.3_

  - [x] 4.3 Update ItemRepository factory and interface
    - Verify ItemRepositoryFactory still works correctly
    - Ensure ItemRepositoryInterface is satisfied by new implementation
    - Test Program.ItemRepo() lazy initialization
    - _Requirements: 4.6_

- [x] 5. Refactor PointRepository to use NestedRepository
  - [x] 5.1 Update PointRepository structure
    - Embed NestedRepository[models.Point] instead of custom implementation
    - Keep resolutionKey and program references
    - Update NewPointRepository to use NewNestedRepository
    - Pass coords.Points map reference to NestedRepository
    - _Requirements: 4.4_

  - [x] 5.2 Remove duplicated CRUD code from PointRepository
    - Delete Get, GetAll, GetAllKeys, Set, Delete, Count implementations
    - Rely on embedded NestedRepository for all CRUD operations
    - _Requirements: 4.4_

  - [x] 5.3 Update PointRepository factory and interface
    - Verify PointRepositoryFactory still works correctly
    - Ensure PointRepositoryInterface is satisfied by new implementation
    - Test Program.PointRepo(resKey) lazy initialization
    - _Requirements: 4.6_

- [x] 6. Refactor SearchAreaRepository to use NestedRepository
  - [x] 6.1 Update SearchAreaRepository structure
    - Embed NestedRepository[models.SearchArea] instead of custom implementation
    - Keep resolutionKey and program references
    - Update NewSearchAreaRepository to use NewNestedRepository
    - Pass coords.SearchAreas map reference to NestedRepository
    - _Requirements: 4.5_

  - [x] 6.2 Remove duplicated CRUD code from SearchAreaRepository
    - Delete Get, GetAll, GetAllKeys, Set, Delete, Count implementations
    - Rely on embedded NestedRepository for all CRUD operations
    - _Requirements: 4.5_

  - [x] 6.3 Update SearchAreaRepository factory and interface
    - Verify SearchAreaRepositoryFactory still works correctly
    - Ensure SearchAreaRepositoryInterface is satisfied by new implementation
    - Test Program.SearchAreaRepo(resKey) lazy initialization
    - _Requirements: 4.6_

- [x] 7. Run tests and verify behavior
  - [x] 7.1 Run existing repository tests
    - Execute all tests in `internal/models/repositories/`
    - Verify base_test.go passes
    - Verify macro_test.go passes
    - Verify program_test.go passes
    - Verify item_test.go passes
    - Verify coordinates_test.go passes
    - Verify integration_test.go passes
    - _Requirements: 5.1_

  - [x] 7.2 Verify error handling behavior
    - Confirm ErrNotFound is returned for missing models
    - Confirm ErrInvalidKey is returned for empty keys
    - Confirm ErrSaveFailed is returned on persistence failures
    - Verify error messages include appropriate context
    - _Requirements: 5.2_

  - [x] 7.3 Verify operational behavior
    - Test case-insensitive key normalization works correctly
    - Verify Set and Delete trigger immediate persistence
    - Confirm thread-safety under concurrent access
    - Test aggregate root persistence for nested repositories
    - _Requirements: 5.3, 5.4, 5.5_

- [x] 8. Update documentation
  - [x] 8.1 Document BaseModel interface
    - Add detailed comments explaining the interface purpose
    - Document GetKey() and SetKey() method contracts
    - Provide implementation examples in comments
    - _Requirements: 6.1, 6.2, 6.3_

  - [x] 8.2 Document repository architecture
    - Add comments explaining BaseRepository vs NestedRepository
    - Document when to use each repository type
    - Explain aggregate root pattern for nested repositories
    - _Requirements: 6.4_
