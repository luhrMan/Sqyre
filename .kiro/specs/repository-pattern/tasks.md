# Implementation Plan

- [x] 1. Create base repository infrastructure
  - Create error definitions and helper types for repository operations
  - Create generic BaseRepository with thread-safe CRUD operations
  - Create decode function signatures and helpers
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 3.1, 3.2, 4.1, 4.2, 4.3, 4.4_

- [x] 2. Implement decode functions
  - [x] 2.1 Create decodeMacro function in repository package
    - Move decode logic from models.Macro to repositories package
    - Integrate with Viper and MacroDecodeHookFunc
    - Add error handling with context
    - _Requirements: 2.2, 2.3, 3.1, 3.2_

  - [x] 2.2 Create decodeProgram function in repository package
    - Move decode logic from models.Program to repositories package
    - Handle nested unmarshalling for Items and Coordinates
    - Add error handling with context
    - _Requirements: 2.2, 2.3, 3.1, 3.2_

- [x] 3. Refactor MacroRepository
  - [x] 3.1 Update MacroRepository to use BaseRepository
    - Replace custom implementation with BaseRepository composition
    - Wire up decodeMacro function
    - Ensure singleton pattern with sync.Once
    - _Requirements: 1.1, 1.2, 1.3, 1.5, 4.5, 6.1, 6.2_

  - [x] 3.2 Remove decode methods from models.Macro
    - Delete Macro.Decode() method
    - Remove commented-out encode/decode functions
    - Keep only NewMacro constructor
    - _Requirements: 2.1, 5.1, 5.2_

- [x] 4. Refactor ProgramRepository
  - [x] 4.1 Update ProgramRepository to use BaseRepository
    - Replace custom implementation with BaseRepository composition
    - Wire up decodeProgram function
    - Ensure singleton pattern with sync.Once
    - _Requirements: 1.1, 1.2, 1.3, 1.5, 4.5, 6.1, 6.2_

  - [x] 4.2 Remove decode methods from models.Program
    - Delete Program.Decode() method
    - Remove commented-out encode/decode functions
    - Keep only NewProgram constructor
    - _Requirements: 2.1, 5.1, 5.2_

- [x] 5. Delete old repository.go file
  - Remove internal/models/repositories/repository.go
  - Verify no remaining references to old implementation
  - _Requirements: 5.1, 5.5_

- [x] 6. Update application initialization
  - [x] 6.1 Update cmd/sqyre/sqyre.go initialization
    - Handle errors from repository initialization
    - Add logging for successful repository loads
    - _Requirements: 3.1, 3.2, 6.1, 6.2, 6.3_

  - [x] 6.2 Update repository calls in UI components
    - Update binders package to handle Get() errors
    - Update ui package components that access repositories
    - Replace direct map access with Get() method calls
    - _Requirements: 1.5, 3.1, 7.1_

  - [x] 6.3 Update repository calls in services
    - Update services package to handle Get() errors
    - Replace direct map access with Get() method calls
    - _Requirements: 1.5, 3.1, 7.1_

- [x] 7. Add repository tests
  - [x] 7.1 Create BaseRepository unit tests
    - Test thread safety with concurrent operations
    - Test CRUD operations
    - Test error handling
    - Test key normalization
    - _Requirements: 1.1, 1.2, 1.5, 3.1, 3.3, 4.1, 4.2, 4.3, 4.4_

  - [x] 7.2 Create MacroRepository tests
    - Test macro-specific decode logic
    - Test singleton initialization
    - Test integration with decode hooks
    - _Requirements: 1.3, 2.2, 2.3, 4.5_

  - [x] 7.3 Create ProgramRepository tests
    - Test program-specific decode logic
    - Test nested structure decoding
    - Test singleton initialization
    - _Requirements: 1.3, 2.2, 2.3, 4.5_

- [x] 8. Add integration tests
  - Create testdata/config.yaml with sample data
  - Test full save and reload cycle
  - Test concurrent repository access
  - _Requirements: 3.5, 4.1, 4.2, 6.1, 6.4_
