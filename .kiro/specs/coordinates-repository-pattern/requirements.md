# Requirements Document

## Introduction

This specification defines the requirements for refactoring the coordinates model to follow the repository pattern established in the Squire application. Currently, coordinates data (Points and SearchAreas) are stored in the `internal/models/coordinates` package and embedded within the Program model. This refactoring will move the coordinate models to `internal/models` and implement a repository pattern similar to the existing ItemRepository, providing consistent data access patterns across the application.

## Glossary

- **Coordinates System**: The collection of Points and SearchAreas that define screen locations for automation
- **Point**: A named screen coordinate (X, Y) used for click and move actions
- **SearchArea**: A named rectangular region (LeftX, TopY, RightX, BottomY) used for image search operations
- **Repository Pattern**: A data access abstraction that provides CRUD operations and persistence management
- **Program**: The parent aggregate that contains Items, Coordinates, and program-specific configurations
- **Resolution Key**: A string identifier in the format "WIDTHxHEIGHT" (e.g., "2560x1440") used to store resolution-specific coordinates

## Requirements

### Requirement 1

**User Story:** As a developer, I want coordinate models (Point and SearchArea) moved to the models package, so that the package structure is consistent with other domain models.

#### Acceptance Criteria

1. WHEN the refactoring is complete, THE Coordinates System SHALL define Point and SearchArea types in `internal/models/coordinates.go`
2. WHEN the refactoring is complete, THE Coordinates System SHALL remove the `internal/models/coordinates` directory
3. WHEN Point and SearchArea types are moved, THE Coordinates System SHALL maintain all existing fields and methods
4. WHEN the models are relocated, THE Coordinates System SHALL update all import statements throughout the codebase

### Requirement 2

**User Story:** As a developer, I want a CoordinatesRepository that follows the same pattern as ItemRepository, so that coordinate data access is consistent with other repositories.

#### Acceptance Criteria

1. WHEN accessing coordinates, THE Coordinates System SHALL provide a CoordinatesRepository in `internal/models/repositories/coordinates.go`
2. WHEN the repository is created, THE Coordinates System SHALL implement Get, GetAll, GetAllKeys, Set, Delete, and Save methods
3. WHEN the repository operates, THE Coordinates System SHALL be scoped to a specific Program and resolution
4. WHEN coordinates are modified, THE Coordinates System SHALL persist changes immediately through the parent Program
5. WHEN accessing Points or SearchAreas, THE Coordinates System SHALL normalize names to lowercase for case-insensitive lookups

### Requirement 3

**User Story:** As a developer, I want the Program model to provide lazy-initialized coordinate repositories, so that coordinate access follows the same pattern as item access.

#### Acceptance Criteria

1. WHEN accessing coordinates from a Program, THE Coordinates System SHALL provide PointRepo() and SearchAreaRepo() methods
2. WHEN a coordinate repository is first accessed, THE Coordinates System SHALL initialize it lazily using a factory pattern
3. WHEN multiple repositories are needed, THE Coordinates System SHALL support separate repositories for Points and SearchAreas
4. WHEN the Program is serialized, THE Coordinates System SHALL maintain the existing Coordinates map structure for backward compatibility

### Requirement 4

**User Story:** As a developer, I want separate repository types for Points and SearchAreas, so that each coordinate type has type-safe access methods.

#### Acceptance Criteria

1. WHEN managing Points, THE Coordinates System SHALL provide a PointRepository with Point-specific operations
2. WHEN managing SearchAreas, THE Coordinates System SHALL provide a SearchAreaRepository with SearchArea-specific operations
3. WHEN repositories are created, THE Coordinates System SHALL share the same resolution key and parent Program reference
4. WHEN either repository saves, THE Coordinates System SHALL persist the entire Program to maintain data consistency

### Requirement 5

**User Story:** As a developer, I want thread-safe coordinate operations, so that concurrent access to coordinates does not cause data corruption.

#### Acceptance Criteria

1. WHEN multiple goroutines access coordinates, THE Coordinates System SHALL use mutex locks for thread safety
2. WHEN read operations occur, THE Coordinates System SHALL use read locks to allow concurrent reads
3. WHEN write operations occur, THE Coordinates System SHALL use write locks to ensure exclusive access
4. WHEN GetAll is called, THE Coordinates System SHALL return a copy of the data to prevent external modification

### Requirement 6

**User Story:** As a developer, I want existing coordinate functionality removed

#### Acceptance Criteria

1. WHEN the refactoring is complete, THE Coordinates System SHALL remove all existing Point methods (GetName)
2. WHEN the refactoring is complete, THE Coordinates System SHALL maintain all existing SearchArea fields
3. WHEN coordinates are accessed, THE Coordinates System SHALL support the existing resolution-based map structure
4. ALL legacy code that accesses THE Coordinates System SHALL be removed

### Requirement 7

**User Story:** As a developer, I want comprehensive tests for the coordinate repositories, so that the implementation is verified to work correctly.

#### Acceptance Criteria

1. WHEN testing coordinate repositories, THE Coordinates System SHALL provide unit tests for PointRepository operations
2. WHEN testing coordinate repositories, THE Coordinates System SHALL provide unit tests for SearchAreaRepository operations
3. WHEN testing persistence, THE Coordinates System SHALL verify that coordinate changes are saved to the parent Program
4. WHEN testing thread safety, THE Coordinates System SHALL verify concurrent access does not cause data races
5. WHEN testing error cases, THE Coordinates System SHALL verify proper error handling for invalid inputs
