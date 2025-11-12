# Requirements Document

## Introduction

This feature addresses a data consistency issue in the repository pattern implementation where CRUD operations do not properly synchronize the map key with the model's internal key field (accessed via `GetKey()`/`SetKey()` from the `BaseModel` interface). When a model is stored or updated with a new key, the model's internal name field may not match the map key, leading to data inconsistencies and potential bugs.

## Glossary

- **BaseModel**: Interface that all persistable models implement, providing `GetKey()` and `SetKey()` methods for accessing the model's unique identifier
- **BaseRepository**: Generic repository for top-level models (Macro, Program) that persists directly to config via Viper
- **NestedRepository**: Generic repository for nested models (Item, Point, SearchArea) that persists by saving the parent aggregate root
- **Map Key**: The string key used to store a model in the repository's internal `map[string]*T`
- **Model Key**: The value returned by the model's `GetKey()` method, typically stored in a `Name` field
- **Key Synchronization**: Ensuring the map key and model key always have the same value
- **Normalized Key**: Lowercase version of a key used for case-insensitive storage and retrieval

## Requirements

### Requirement 1

**User Story:** As a developer using the repository pattern, I want the model's internal key field to automatically synchronize with the map key when storing or updating models, so that data consistency is maintained without manual intervention.

#### Acceptance Criteria

1. WHEN a model is stored using `Set(key, model)`, THE BaseRepository SHALL call `model.SetKey(key)` before storing the model in the map
2. WHEN a model is stored using `Set(key, model)`, THE NestedRepository SHALL call `model.SetKey(key)` before storing the model in the map
3. WHEN a model is retrieved using `Get(key)`, THE BaseRepository SHALL return a model where `model.GetKey()` equals the normalized key
4. WHEN a model is retrieved using `Get(key)`, THE NestedRepository SHALL return a model where `model.GetKey()` equals the normalized key
5. WHEN a model is renamed by calling `Set(newKey, existingModel)`, THE repository SHALL update the model's internal key field to match the new key

### Requirement 2

**User Story:** As a developer, I want the repository to handle key normalization consistently, so that case-insensitive lookups work correctly while preserving the original case in the model.

#### Acceptance Criteria

1. WHEN a key is provided to `Set()`, THE repository SHALL normalize the key to lowercase for map storage
2. WHEN a key is provided to `Set()`, THE repository SHALL call `SetKey()` with the normalized lowercase key
3. WHEN a model is retrieved, THE model's `GetKey()` method SHALL return the normalized lowercase key
4. WHEN multiple operations use different case variations of the same key, THE repository SHALL treat them as the same key

### Requirement 3

**User Story:** As a developer, I want existing tests to validate the key synchronization behavior, so that I can verify the fix works correctly and prevent regressions.

#### Acceptance Criteria

1. WHEN the `Set()` method is called in tests, THE tests SHALL verify that `model.GetKey()` equals the provided key
2. WHEN a model is renamed in tests, THE tests SHALL verify that the old key is removed and the new key contains the updated model
3. WHEN tests run after the fix, THE BaseRepository tests SHALL pass without modification
4. WHEN tests run after the fix, THE NestedRepository tests SHALL pass without modification
5. IF new test cases are needed, THEN THE repository SHALL include tests that explicitly verify key synchronization

### Requirement 4

**User Story:** As a developer, I want the key synchronization to work with all existing model types, so that the fix applies consistently across the codebase.

#### Acceptance Criteria

1. WHEN the fix is applied, THE BaseRepository SHALL synchronize keys for Macro models
2. WHEN the fix is applied, THE BaseRepository SHALL synchronize keys for Program models
3. WHEN the fix is applied, THE NestedRepository SHALL synchronize keys for Item models
4. WHEN the fix is applied, THE NestedRepository SHALL synchronize keys for Point models
5. WHEN the fix is applied, THE NestedRepository SHALL synchronize keys for SearchArea models
