<!-- # Implementation Plan

- [ ] 1. Implement key synchronization in BaseRepository
  - Modify the `Set()` method in `internal/models/repositories/base.go` to call `SetKey()` before storing the model
  - Add type assertion to check if model implements `BaseModel` interface
  - Call `baseModel.SetKey(normalizedKey)` to synchronize the model's internal key with the map key
  - Ensure the synchronization happens within the existing mutex lock
  - _Requirements: 1.1, 1.5, 2.1, 2.2, 4.1, 4.2_

- [ ] 2. Implement key synchronization in NestedRepository
  - Modify the `Set()` method in `internal/models/repositories/nested.go` to call `SetKey()` before storing the model
  - Add identical type assertion logic as BaseRepository
  - Call `baseModel.SetKey(normalizedKey)` to synchronize the model's internal key with the map key
  - Ensure the synchronization happens within the existing mutex lock
  - _Requirements: 1.2, 1.5, 2.1, 2.2, 4.3, 4.4, 4.5_ -->

- [x] 3. Verify existing tests pass with the changes
  - Run the existing BaseRepository tests in `internal/models/repositories/base_test.go`
  - Run the existing MacroRepository tests in `internal/models/repositories/macro_test.go`
  - Run the existing ProgramRepository tests in `internal/models/repositories/program_test.go`
  - Verify all tests pass without modification
  - _Requirements: 3.3, 3.4_

- [x] 4. Add key synchronization tests for BaseRepository
  - [x] 4.1 Create test for basic key synchronization on Set operation
    - Create a test that verifies `model.GetKey()` equals the provided key after `Set()`
    - Test with a simple model that implements BaseModel
    - Verify the model's internal Name field is updated
    - _Requirements: 1.1, 1.3, 3.1, 3.5_
  
  - [x] 4.2 Create test for case normalization
    - Create a test that verifies keys are normalized to lowercase
    - Test `Set("MixedCase", model)` results in `model.GetKey()` returning "mixedcase"
    - Verify case-insensitive retrieval works correctly
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.5_
  
  - [x] 4.3 Create test for rename operation
    - Create a test that stores a model with one key, then calls `Set()` with a different key
    - Verify the old key no longer exists in the repository
    - Verify the new key contains the model with updated internal key
    - Verify the model's data (non-key fields) is preserved
    - _Requirements: 1.5, 3.2, 3.5_

- [ ] 5. Add key synchronization tests for NestedRepository
  - [x] 5.1 Create test for Item key synchronization
    - Create a Program with an ItemRepository
    - Create an Item and call `Set()` with a different key than the item's Name
    - Verify the item's internal Name field is updated to match the key
    - Verify the parent Program is saved correctly
    - _Requirements: 1.2, 1.4, 4.3, 3.5_
  
  - [x] 5.2 Create test for Point key synchronization
    - Create a Program with a PointRepository for a specific resolution
    - Create a Point and call `Set()` with a different key than the point's Name
    - Verify the point's internal Name field is updated to match the key
    - Verify the parent Program is saved correctly
    - _Requirements: 1.2, 1.4, 4.4, 3.5_
  
  - [x] 5.3 Create test for SearchArea key synchronization
    - Create a Program with a SearchAreaRepository for a specific resolution
    - Create a SearchArea and call `Set()` with a different key than the area's Name
    - Verify the area's internal Name field is updated to match the key
    - Verify the parent Program is saved correctly
    - _Requirements: 1.2, 1.4, 4.5, 3.5_

- [ ]* 6. Add integration tests for real-world scenarios
  - [ ]* 6.1 Test Macro rename scenario
    - Create a Macro with MacroRepository
    - Rename it by calling `Set()` with a new key
    - Verify the macro can be retrieved by the new key
    - Verify the old key no longer exists
    - _Requirements: 4.1_
  
  - [ ]* 6.2 Test Program with nested Items rename scenario
    - Create a Program with multiple Items
    - Rename an Item using ItemRepository
    - Verify the Item's key is synchronized
    - Verify other Items are unaffected
    - Verify the Program persists correctly
    - _Requirements: 4.2, 4.3_
  
  - [ ]* 6.3 Test concurrent Set operations
    - Create multiple goroutines that call `Set()` concurrently
    - Verify all models have synchronized keys after operations complete
    - Verify no race conditions occur
    - Verify thread safety is maintained
    - _Requirements: 1.1, 1.2_
