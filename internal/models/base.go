package models

// BaseModel defines the interface that all persistable models must implement.
// This interface enables generic repository operations by providing a standard
// way to access and modify the model's unique identifier.
//
// # Purpose
//
// BaseModel serves as the foundation for the repository pattern in Squire, enabling
// type-safe, generic CRUD operations across different model types. By implementing
// this interface, any model can be managed by BaseRepository or NestedRepository
// without requiring custom repository implementations.
//
// The interface abstracts the concept of a "key" (unique identifier) from the
// underlying storage mechanism. This allows models to use any field as their key
// (typically a Name field) while repositories handle normalization, persistence,
// and retrieval consistently.
//
// # Design Rationale
//
// - Interface over struct: Provides flexibility in how models store their key
// - Minimal contract: Only requires what's essential for repository operations
// - Generic-friendly: Works seamlessly with Go generics for type-safe repositories
// - Case-insensitive: Repositories normalize keys to lowercase for consistent lookups
//
// # Implementation Requirements
//
// Models implementing BaseModel must:
// 1. Have a field that serves as the unique identifier (typically "Name")
// 2. Return that field's value in GetKey()
// 3. Update that field's value in SetKey()
// 4. Ensure the key field is exported for serialization (e.g., YAML, JSON)
//
// # Example Implementation
//
// Basic implementation using a Name field:
//
//	type MyModel struct {
//	    Name        string  // The key field
//	    Description string
//	    Value       int
//	}
//
//	func (m *MyModel) GetKey() string {
//	    return m.Name
//	}
//
//	func (m *MyModel) SetKey(key string) {
//	    m.Name = key
//	}
//
// # Usage with Repositories
//
// Top-level models (stored directly in config):
//
//	// Create a repository for top-level models
//	repo := repositories.NewBaseRepository[MyModel](
//	    "mymodels",                    // Config key
//	    decodeMyModel,                 // Decode function
//	    func() *MyModel { return &MyModel{} },
//	)
//
//	// Store a model - SetKey is called automatically
//	model := &MyModel{Description: "test"}
//	repo.Set("mykey", model)           // Calls model.SetKey("mykey")
//
//	// Retrieve a model - GetKey is used for lookup
//	retrieved, _ := repo.Get("mykey")  // Uses model.GetKey() internally
//	fmt.Println(retrieved.GetKey())    // Output: "mykey"
//
// Nested models (stored within an aggregate root):
//
//	// Create a repository for nested models
//	itemRepo := repositories.NewNestedRepository[Item](
//	    program.Items,                 // Parent's map
//	    program.GetKey(),              // Context
//	    func() error {                 // Save parent
//	        return programRepo.Set(program.GetKey(), program)
//	    },
//	)
//
//	// Operations work the same way
//	item := &Item{Description: "Health Potion"}
//	itemRepo.Set("health-potion", item)
//
// # Current Implementations
//
// The following models in Squire implement BaseModel:
// - Macro: Top-level model for automation macros
// - Program: Top-level model for game-specific configurations
// - Item: Nested model within Program
// - Point: Nested model within Program/Coordinates
// - SearchArea: Nested model within Program/Coordinates
//
// # Adding New Models
//
// To add a new model that works with repositories:
//
//	// 1. Define your model struct with a key field
//	type NewModel struct {
//	    Name   string  // Key field
//	    Config string
//	}
//
//	// 2. Implement BaseModel interface
//	func (n *NewModel) GetKey() string    { return n.Name }
//	func (n *NewModel) SetKey(key string) { n.Name = key }
//
//	// 3. Create a repository (BaseRepository or NestedRepository)
//	// 4. Use standard CRUD operations: Get, Set, Delete, GetAll, etc.
type BaseModel interface {
	// GetKey returns the unique identifier for this model instance.
	//
	// Contract:
	// - MUST return a non-empty string for persisted models
	// - SHOULD return the same value across multiple calls (unless SetKey is called)
	// - The returned value is used as the storage key in repositories
	// - Repositories normalize keys to lowercase, so "MyKey" and "mykey" are equivalent
	//
	// Implementation note:
	// Typically returns the value of a Name field, but can return any field
	// that serves as a unique identifier for the model.
	//
	// Example:
	//	func (m *Macro) GetKey() string {
	//	    return m.Name
	//	}
	GetKey() string

	// SetKey updates the unique identifier for this model instance.
	//
	// Contract:
	// - MUST update the model's key field to the provided value
	// - Called by repositories before storing to ensure key consistency
	// - The key parameter may be different from the current GetKey() value
	// - Should handle empty strings gracefully (though repositories validate this)
	//
	// Implementation note:
	// Typically updates a Name field, ensuring the model's internal state
	// matches the key used for storage.
	//
	// Example:
	//	func (m *Macro) SetKey(key string) {
	//	    m.Name = key
	//	}
	SetKey(key string)
}
