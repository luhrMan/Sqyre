# Implementation Plan

- [x] 1. Move coordinate models to internal/models package
  - Create `internal/models/coordinates.go` with Point and SearchArea types
  - Copy Point struct with Name, X, Y fields
  - Copy SearchArea struct with Name, LeftX, TopY, RightX, BottomY fields
  - Copy Coordinates struct with Points and SearchAreas maps
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. Create repository interfaces in Program model
  - Add PointRepositoryInterface in `internal/models/program.go`
  - Add SearchAreaRepositoryInterface in `internal/models/program.go`
  - Define Get, GetAll, GetAllKeys, Set, Delete, Save, Count methods for both interfaces
  - Add factory function variables (PointRepositoryFactory, SearchAreaRepositoryFactory)
  - _Requirements: 2.1, 2.2, 3.1, 3.2_

- [x] 3. Implement PointRepository
- [x] 3.1 Create PointRepository struct
  - Create `internal/models/repositories/coordinates.go`
  - Define PointRepository with mu, points map reference, resolutionKey, program fields
  - Implement NewPointRepository constructor with resolution key parameter
  - Initialize Coordinates map entry if it doesn't exist
  - _Requirements: 2.3, 2.4, 3.3_

- [x] 3.2 Implement PointRepository CRUD methods
  - Implement Get() with case-insensitive lookup and ErrNotFound handling
  - Implement GetAll() returning a copy of the points map
  - Implement GetAllKeys() returning sorted slice of point names
  - Implement Set() with lowercase normalization and immediate Save()
  - Implement Delete() with immediate Save()
  - Implement Count() returning number of points
  - _Requirements: 2.1, 2.2, 2.5, 5.1, 5.2, 5.3, 5.4_

- [x] 3.3 Implement PointRepository Save method
  - Implement Save() that persists through ProgramRepository
  - Use ProgramRepo().Set() to save the entire parent Program
  - Return wrapped errors with context (program name, resolution)
  - _Requirements: 2.4, 4.4_

- [x] 4. Implement SearchAreaRepository
- [x] 4.1 Create SearchAreaRepository struct
  - Define SearchAreaRepository with mu, searchAreas map reference, resolutionKey, program fields
  - Implement NewSearchAreaRepository constructor
  - Initialize Coordinates map entry if it doesn't exist
  - _Requirements: 2.3, 4.1, 4.3_

- [x] 4.2 Implement SearchAreaRepository CRUD methods
  - Implement Get() with case-insensitive lookup
  - Implement GetAll() returning a copy
  - Implement GetAllKeys() returning sorted slice
  - Implement Set() with lowercase normalization and immediate Save()
  - Implement Delete() with immediate Save()
  - Implement Count()
  - _Requirements: 2.1, 2.2, 2.5, 4.2, 5.1, 5.2, 5.3, 5.4_

- [x] 4.3 Implement SearchAreaRepository Save method
  - Implement Save() through ProgramRepository
  - Return wrapped errors with context
  - _Requirements: 2.4, 4.4_

- [x] 5. Add factory initialization
  - Add init() function in `internal/models/repositories/coordinates.go`
  - Set PointRepositoryFactory to NewPointRepository
  - Set SearchAreaRepositoryFactory to NewSearchAreaRepository
  - _Requirements: 3.2_

- [x] 6. Update Program model with repository accessors
- [x] 6.1 Add repository fields to Program struct
  - Add pointRepos map[string]PointRepositoryInterface field
  - Add searchAreaRepos map[string]SearchAreaRepositoryInterface field
  - Add repoMu sync.Mutex field (or reuse existing itemRepoMu)
  - _Requirements: 3.1, 3.3_

- [x] 6.2 Implement PointRepo accessor method
  - Create PointRepo(resolutionKey string) method
  - Implement lazy initialization with mutex protection
  - Initialize pointRepos map if nil
  - Use factory to create repository if not exists
  - Add panic check for uninitialized factory
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 6.3 Implement SearchAreaRepo accessor method
  - Create SearchAreaRepo(resolutionKey string) method
  - Implement lazy initialization with mutex protection
  - Initialize searchAreaRepos map if nil
  - Use factory to create repository if not exists
  - Add panic check for uninitialized factory
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 7. Update all import statements
  - Search codebase for `"Squire/internal/models/coordinates"` imports
  - Replace with `"Squire/internal/models"` where Point or SearchArea are used
  - Update any type references from `coordinates.Point` to `models.Point`
  - Update any type references from `coordinates.SearchArea` to `models.SearchArea`
  - _Requirements: 1.4_

- [x] 8. Write unit tests for PointRepository
  - Create `internal/models/repositories/coordinates_test.go`
  - Test Get() with valid and invalid names
  - Test Set() creates and updates points
  - Test Delete() removes points
  - Test GetAll() returns a copy
  - Test GetAllKeys() returns sorted names
  - Test Count() returns correct count
  - Test case-insensitive name handling
  - Test error conditions (empty names, not found)
  - _Requirements: 7.1, 7.3, 7.5_

- [x] 9. Write unit tests for SearchAreaRepository
  - Test Get() with valid and invalid names
  - Test Set() creates and updates search areas
  - Test Delete() removes search areas
  - Test GetAll() returns a copy
  - Test GetAllKeys() returns sorted names
  - Test Count() returns correct count
  - Test case-insensitive name handling
  - Test error conditions
  - _Requirements: 7.2, 7.3, 7.5_

- [x] 10. Write thread safety tests
  - Test concurrent reads to PointRepository
  - Test concurrent writes to PointRepository
  - Test concurrent reads to SearchAreaRepository
  - Test concurrent writes to SearchAreaRepository
  - Run tests with `go test -race` to detect race conditions
  - _Requirements: 7.4_

- [x] 11. Write integration tests
  - Test Program.PointRepo() lazy initialization
  - Test Program.SearchAreaRepo() lazy initialization
  - Test multiple resolution keys work independently
  - Test Save() persists through ProgramRepository
  - Test backward compatibility with direct Coordinates map access
  - Verify changes are written to config file
  - _Requirements: 7.3, 7.4_
