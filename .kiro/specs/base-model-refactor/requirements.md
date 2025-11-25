# Requirements Document

## Introduction

This feature refactors the repository pattern implementation to eliminate code duplication by properly leveraging a BaseModel interface and generic repository pattern. Currently, the codebase has a `BaseModel` struct and `BaseRepository[T]` generic implementation, but several repositories (ItemRepository, PointRepository, SearchAreaRepository) duplicate CRUD logic instead of using the generic base. This refactor will consolidate the repository pattern, making the codebase more maintainable and reducing the risk of inconsistencies across different model types.

## Glossary

- **BaseModel**: An interface that all domain models (Macro, Program, Item, Point, SearchArea) must implement to work with the generic repository pattern
- **Repository**: A data access layer component that provides CRUD operations for a specific model type
- **Generic Repository**: A type-parameterized repository implementation that can work with any model implementing BaseModel
- **Aggregate Root**: A domain model that owns and manages child entities (e.g., Program owns Items, Points, SearchAreas)
- **Nested Repository**: A repository that manages entities within an aggregate root's context (e.g., ItemRepository manages Items within a Program)
- **Viper**: The configuration management library used for persistence
- **CRUD**: Create, Read, Update, Delete operations

## Requirements

### Requirement 1

**User Story:** As a developer, I want all models to implement a common BaseModel interface, so that I can use generic repository operations consistently across all model types

#### Acceptance Criteria

1. THE System SHALL define a BaseModel interface with methods GetKey() string and SetKey(string)
2. THE System SHALL update the Macro model to implement the BaseModel interface
3. THE System SHALL update the Program model to implement the BaseModel interface
4. THE System SHALL update the Item model to implement the BaseModel interface
5. THE System SHALL update the Point model to implement the BaseModel interface
6. THE System SHALL update the SearchArea model to implement the BaseModel interface

### Requirement 2

**User Story:** As a developer, I want the BaseRepository to work with any model implementing BaseModel, so that I can eliminate duplicated repository code

#### Acceptance Criteria

1. THE System SHALL update BaseRepository type constraint from `models.BaseModel` struct to `BaseModel` interface
2. THE System SHALL ensure BaseRepository provides Get, GetAll, GetAllKeys, Set, Delete, Save, Reload, and Count operations
3. WHEN a repository operation is called, THE System SHALL use the model's GetKey() method for key normalization
4. THE System SHALL maintain thread-safe operations using sync.RWMutex in BaseRepository

### Requirement 3

**User Story:** As a developer, I want MacroRepository and ProgramRepository to use BaseRepository without modification, so that top-level models have consistent behavior

#### Acceptance Criteria

1. THE System SHALL verify MacroRepository correctly uses BaseRepository with the Macro model
2. THE System SHALL verify ProgramRepository correctly uses BaseRepository with the Program model
3. THE System SHALL ensure singleton pattern initialization works correctly for both repositories
4. THE System SHALL maintain backward compatibility with existing MacroRepo() and ProgramRepo() functions

### Requirement 4

**User Story:** As a developer, I want nested repositories (Item, Point, SearchArea) to leverage BaseRepository functionality, so that I eliminate code duplication while maintaining aggregate root relationships

#### Acceptance Criteria

1. THE System SHALL create a NestedRepository type that embeds BaseRepository and adds aggregate root context
2. WHEN a nested repository saves, THE System SHALL persist changes through the parent aggregate root
3. THE System SHALL refactor ItemRepository to use NestedRepository pattern
4. THE System SHALL refactor PointRepository to use NestedRepository pattern
5. THE System SHALL refactor SearchAreaRepository to use NestedRepository pattern
6. THE System SHALL maintain existing repository factory functions for dependency injection

### Requirement 5

**User Story:** As a developer, I want all existing tests to pass after the refactor, so that I can verify the changes don't break existing functionality

#### Acceptance Criteria

1. WHEN the refactor is complete, THE System SHALL pass all existing repository tests
2. THE System SHALL maintain existing error handling behavior (ErrNotFound, ErrInvalidKey, ErrSaveFailed)
3. THE System SHALL preserve case-insensitive key normalization behavior
4. THE System SHALL maintain immediate persistence on Set and Delete operations
5. THE System SHALL preserve thread-safety guarantees in all repository operations

### Requirement 6

**User Story:** As a developer, I want clear documentation of the BaseModel interface contract, so that I can easily add new models in the future

#### Acceptance Criteria

1. THE System SHALL document the BaseModel interface with clear comments explaining its purpose
2. THE System SHALL document the GetKey() and SetKey() method contracts
3. THE System SHALL provide examples in comments showing how to implement BaseModel for new types
4. THE System SHALL document the relationship between BaseModel, BaseRepository, and NestedRepository
