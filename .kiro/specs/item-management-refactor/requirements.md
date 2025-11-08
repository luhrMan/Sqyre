# Requirements Document

## Introduction

This document defines requirements for refactoring item management in Squire to follow the established repository pattern. Currently, item operations are implemented as methods directly on the Program model, which mixes business logic with data access concerns. The goal is to create a consistent ItemRepository that provides the same clean interface as MacroRepository and ProgramRepository, while maintaining the aggregate relationship where Items belong to Programs.

## Glossary

- **Item**: A game object with properties like name, grid size, tags, stack max, and merchant
- **Program**: An aggregate root that contains a collection of Items
- **ItemRepository**: A data access layer that manages Item persistence within a Program context
- **Aggregate Root**: A domain-driven design pattern where one entity (Program) controls access to related entities (Items)
- **Repository Pattern**: A standardized interface for data access operations (Get, Set, Delete, etc.)

## Requirements

### Requirement 1: Item Repository Interface

**User Story:** As a developer, I want item operations to follow the same repository pattern as macros and programs, so that data access is consistent across the codebase.

#### Acceptance Criteria

1. THE ItemRepository SHALL implement the standard Repository interface with methods: Get, GetAll, GetAllKeys, Set, Delete, Save
2. THE ItemRepository SHALL be scoped to a specific Program (items belong to programs)
3. THE ItemRepository SHALL use lowercase keys for case-insensitive item name access
4. THE ItemRepository SHALL return errors when items are not found instead of auto-creating them
5. WHERE an item does not exist, THE ItemRepository SHALL return ErrNotFound

### Requirement 2: Program-Scoped Repository Access

**User Story:** As a developer, I want to access items through their parent program, so that the aggregate boundary is maintained.

#### Acceptance Criteria

1. THE Program SHALL provide an ItemsRepo() method that returns an ItemRepository instance
2. THE ItemRepository SHALL operate only on items within its parent Program
3. WHEN a Program is loaded, THE ItemRepository SHALL be initialized with the Program's items
4. THE ItemRepository SHALL save changes back to the parent Program's Items map
5. THE Program SHALL remain the aggregate root for all item operations

### Requirement 3: Clean Item Operations

**User Story:** As a developer, I want clean CRUD operations for items, so that item management is straightforward and error-free.

#### Acceptance Criteria

1. THE ItemRepository SHALL provide Get(name string) to retrieve a single item by name
2. THE ItemRepository SHALL provide GetAll() to retrieve all items as a map
3. THE ItemRepository SHALL provide GetAllKeys() to retrieve sorted item names
4. THE ItemRepository SHALL provide Set(name string, item *Item) to create or update an item
5. THE ItemRepository SHALL provide Delete(name string) to remove an item

<!-- ### Requirement 4: Backward Compatibility

**User Story:** As a developer, I want existing code to continue working during the migration, so that the refactor can be done incrementally.

#### Acceptance Criteria

1. THE Program SHALL maintain existing helper methods (GetItemsAsStringSlice, SortItemsByName) during migration
2. THE ItemRepository SHALL provide equivalent functionality to existing Program methods
3. WHEN migrating code, THE existing method calls SHALL be replaceable one-at-a-time
4. THE migration SHALL not break existing UI components or services
5. WHERE possible, THE old methods SHALL delegate to the new ItemRepository -->

### Requirement 5: Error Handling

**User Story:** As a developer, I want clear error handling for item operations, so that I can diagnose and handle failures appropriately.

#### Acceptance Criteria

1. WHEN an item is not found, THE ItemRepository SHALL return ErrNotFound with the item name
2. WHEN an item name is empty, THE ItemRepository SHALL return ErrInvalidKey
3. WHEN adding a duplicate item, THE ItemRepository SHALL return an error indicating the item exists
4. THE ItemRepository SHALL validate item data before saving
5. THE ItemRepository SHALL log operations with appropriate context

### Requirement 6: Thread Safety

**User Story:** As a developer, I want item operations to be thread-safe, so that concurrent access does not corrupt data.

#### Acceptance Criteria

1. THE ItemRepository SHALL use sync.RWMutex for protecting concurrent operations
2. WHEN multiple goroutines access items, THE ItemRepository SHALL prevent race conditions
3. THE ItemRepository SHALL use read locks for Get and GetAll operations
4. THE ItemRepository SHALL use write locks for Set and Delete operations
5. THE ItemRepository SHALL coordinate with Program-level saves for persistence

### Requirement 7: Special Item Operations

**User Story:** As a developer, I want to maintain special item query operations, so that UI components can display items with program context.

#### Acceptance Criteria

1. THE ItemRepository SHALL provide GetAllWithProgramPrefix() to return items with "program|item" format
2. THE ItemRepository SHALL provide GetAllSorted() to return alphabetically sorted item names
3. THE ItemRepository SHALL support filtering items by tags or other properties
4. THE ItemRepository SHALL maintain the existing delimiter pattern (program|item)
5. WHERE items are displayed in UI, THE ItemRepository SHALL provide formatted names

### Requirement 8: Persistence Integration

**User Story:** As a developer, I want item changes to persist correctly, so that data is not lost between sessions.

#### Acceptance Criteria

1. WHEN an item is modified, THE ItemRepository SHALL update the parent Program's Items map
2. WHEN Save() is called, THE ItemRepository SHALL trigger a Program save through ProgramRepository
3. THE ItemRepository SHALL maintain referential integrity with the Program's Items map
4. THE ItemRepository SHALL not create separate persistence files for items
5. THE Program SHALL remain the unit of persistence for all its items
