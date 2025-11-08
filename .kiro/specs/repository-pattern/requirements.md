# Requirements Document

## Introduction

This document defines requirements for implementing a standardized repository pattern in Squire. The current repository implementation has inconsistencies, commented-out code, and lacks clear separation of concerns between models and data access. The goal is to establish a clean, consistent pattern for data persistence that can be easily extended for new domain models.

## Glossary

- **Repository**: A data access layer that abstracts persistence operations for domain models
- **Domain Model**: Business entities like Macro and Program that represent core application concepts
- **Viper**: Configuration management library used for YAML serialization/deserialization
- **Singleton Repository**: A repository instance that is initialized once and reused throughout the application lifecycle
- **Decode Hook**: Custom deserialization logic for complex types during Viper unmarshalling

## Requirements

### Requirement 1: Repository Interface Standardization

**User Story:** As a developer, I want a consistent repository interface across all domain models, so that data access patterns are predictable and maintainable.

#### Acceptance Criteria

1. THE Repository SHALL define a standard interface with methods: Get, GetAll, Set, Delete, Save, and Load
2. THE Repository SHALL use Go generics to support type-safe operations for any domain model
3. THE Repository SHALL provide singleton access through factory functions (e.g., MacroRepo(), ProgramRepo())
4. THE Repository SHALL handle all persistence operations without exposing Viper implementation details to callers
5. WHERE a model does not exist, THE Repository SHALL return an error rather than creating empty instances automatically

### Requirement 2: Separation of Concerns

**User Story:** As a developer, I want domain models to be independent of persistence logic, so that models focus on business logic and repositories handle data access.

#### Acceptance Criteria

1. THE Domain Model SHALL NOT contain Encode or Decode methods
2. THE Repository SHALL handle all serialization and deserialization logic
3. THE Repository SHALL use decode hooks for complex type unmarshalling without model involvement
4. THE Domain Model SHALL define constructor functions (e.g., NewMacro, NewProgram) for creating new instances
5. THE Repository SHALL call model constructors when creating new instances

### Requirement 3: Error Handling

**User Story:** As a developer, I want clear error handling in repository operations, so that I can diagnose and handle persistence failures appropriately.

#### Acceptance Criteria

1. WHEN a repository operation fails, THE Repository SHALL return a descriptive error
2. THE Repository SHALL log errors with context including operation type and model name
3. WHEN loading data fails, THE Repository SHALL return an error rather than silently creating empty data
4. THE Repository SHALL validate data integrity before saving
5. WHERE decode operations fail for individual items, THE Repository SHALL log the failure and continue loading other items

### Requirement 4: Thread Safety

**User Story:** As a developer, I want repository operations to be thread-safe, so that concurrent access does not corrupt data.

#### Acceptance Criteria

1. THE Repository SHALL use sync.RWMutex for protecting concurrent read/write operations
2. WHEN multiple goroutines access the repository, THE Repository SHALL prevent race conditions
3. THE Repository SHALL use read locks for Get and GetAll operations
4. THE Repository SHALL use write locks for Set, Delete, and Save operations
5. THE Repository SHALL initialize singletons using sync.Once to prevent duplicate initialization

### Requirement 5: Clean Code Structure

**User Story:** As a developer, I want repository code to be clean and maintainable, so that future modifications are straightforward.

#### Acceptance Criteria

1. THE Repository SHALL remove all commented-out code
2. THE Repository SHALL use consistent naming conventions across all repository implementations
3. THE Repository SHALL document public methods with clear godoc comments
4. THE Repository SHALL separate generic repository logic from model-specific logic
5. THE Repository SHALL use lowercase keys consistently when storing and retrieving data

### Requirement 6: Initialization and Configuration

**User Story:** As a developer, I want repositories to initialize cleanly at application startup, so that configuration loading is reliable and traceable.

#### Acceptance Criteria

1. WHEN the application starts, THE Repository SHALL load existing data from config.yaml
2. THE Repository SHALL log successful initialization with model counts
3. WHERE config.yaml does not exist, THE Repository SHALL initialize with empty data structures
4. THE Repository SHALL validate configuration file format before loading
5. THE Repository SHALL provide a Reload method to refresh data from disk without restarting

### Requirement 7: CRUD Operations

**User Story:** As a developer, I want complete CRUD operations for all domain models, so that I can manage data throughout the application lifecycle.

#### Acceptance Criteria

1. THE Repository SHALL provide Get(key string) to retrieve a single model by key
2. THE Repository SHALL provide GetAll() to retrieve all models as a map
3. THE Repository SHALL provide Set(key string, model) to create or update a model
4. THE Repository SHALL provide Delete(key string) to remove a model
5. THE Repository SHALL provide Save() to persist all changes to disk immediately
