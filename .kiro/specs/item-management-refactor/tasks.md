# Implementation Plan

- [x] 1. Create ItemRepository infrastructure
  - Create `internal/models/repositories/item.go` with ItemRepository struct
  - Implement NewItemRepository constructor that takes a Program reference
  - Add ItemRepository fields: mu (RWMutex), items (reference to Program.Items), programName, program
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 6.1_

- [x] 2. Implement core ItemRepository methods
- [x] 2.1 Implement Get and GetAll methods
  - Write Get(name string) with case-insensitive lookup and ErrNotFound handling
  - Write GetAll() that returns a copy of the items map
  - Write GetAllKeys() that returns sorted item names
  - Write Count() that returns the number of items
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 3.1, 3.2, 3.3, 5.1, 6.2, 6.3_

- [x] 2.2 Implement Set and Delete methods
  - Write Set(name string, item *Item) with validation and save
  - Write Delete(name string) with save
  - Write Save() that calls ProgramRepository.Set() for parent program
  - Add proper error handling with program context
  - _Requirements: 1.1, 3.4, 3.5, 5.2, 5.4, 5.5, 6.2, 6.4, 8.1, 8.2, 8.3, 8.5_

- [x] 3. Implement special ItemRepository methods
  - Write GetAllWithProgramPrefix() that formats items as "program|item"
  - Write GetAllSorted() that returns alphabetically sorted item names
  - Use config.ProgramDelimiter for the delimiter
  - _Requirements: 7.1, 7.2, 7.4, 7.5_

- [x] 4. Integrate ItemRepository with Program model
  - Add itemRepo field and itemRepoMu mutex to Program struct
  - Implement Program.Items() method with lazy initialization
  - Ensure thread-safe initialization of ItemRepository
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 5. Create wrapper methods for backward compatibility
  - Update Program.GetItem() to delegate to Items().Get()
  - Update Program.SetItem() to delegate to Items().Set()
  - Update Program.AddItem() to delegate to Items().Set() with duplicate check
  - Update Program.DeleteItem() to delegate to Items().Delete()
  - Update Program.GetItemsAsStringSlice() to delegate to Items().GetAllKeys()
  - Update Program.SortItemsByName() to delegate to Items().GetAllSorted()
  - Update Program.GetItemsWithAppendedProgramName() to delegate to Items().GetAllWithProgramPrefix()
  - Add deprecation comments to old methods
  - _Requirements: 4.1, 4.2, 4.3, 4.5_

- [x] 6. Add ItemRepository tests
- [x] 6.1 Create unit tests for ItemRepository
  - Test Get with existing items, case insensitivity, and not found errors
  - Test GetAll returns a copy of items
  - Test GetAllKeys returns sorted names
  - Test Set creates and updates items
  - Test Delete removes items
  - Test Count returns correct number
  - Test empty name validation returns ErrInvalidKey
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 3.1, 3.2, 3.3, 3.4, 3.5, 5.1, 5.2_

- [x] 6.2 Create thread safety tests
  - Test concurrent Get operations
  - Test concurrent Set operations
  - Test mixed concurrent reads and writes
  - Verify no race conditions with go test -race
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 6.3 Create special method tests
  - Test GetAllWithProgramPrefix formats correctly
  - Test GetAllSorted returns alphabetical order
  - Verify delimiter usage matches config.ProgramDelimiter
  - _Requirements: 7.1, 7.2, 7.4, 7.5_

- [x] 6.4 Create integration tests
  - Test creating Program with items and saving through ProgramRepository
  - Test reloading Program and accessing items through ItemRepository
  - Test modifying items and verifying persistence
  - Test that ItemRepository.Save() triggers Program save
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 7. Update UI components to use ItemRepository
  - Search for Program.GetItem() calls in ui/ directory
  - Replace with program.ItemsRepo().Get() pattern
  - Update error handling to check for ErrNotFound
  - Search for Program.SetItem() and Program.AddItem() calls
  - Replace with program.ItemsRepo().Set() pattern
  - Search for Program.GetItemsAsStringSlice() calls
  - Replace with program.ItemsRepo().GetAllKeys() pattern
  - _Requirements: 4.2, 4.3, 4.4, 5.1_

- [x] 8. Update services to use ItemRepository
  - Search for item-related Program method calls in internal/services/
  - Replace with ItemRepository equivalents
  - Update error handling appropriately
  - _Requirements: 4.2, 4.3, 4.4, 5.1_

- [x] 9. Update binders to use ItemRepository
  - Search for item-related Program method calls in binders/
  - Replace with ItemRepository equivalents
  - Update data binding logic if needed
  - _Requirements: 4.2, 4.3, 4.4_

- [x] 10. Remove deprecated Program methods
  - Remove Program.GetItem() method
  - Remove Program.SetItem() method
  - Remove Program.NewItem() method
  - Remove Program.AddItem() method
  - Remove Program.DeleteItem() method
  - Remove Program.GetItemsAsStringSlice() method
  - Remove Program.SortItemsByName() method
  - Remove Program.GetItemsWithAppendedProgramName() method
  - Remove Program.GetItemsMap() method (direct map access should use Items().GetAll())
  - Verify no remaining references to removed methods
  - _Requirements: 4.4, 4.5_
