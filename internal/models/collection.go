package models

// Collection is a named grid over a program Search Area.
// Cell geometry is derived at runtime from the linked search area bounds
// divided by Rows × Cols (1-based indexing in UI and CoordinateRef ranges).
type Collection struct {
	Name       string
	SearchArea string // Search Area name in the same program
	Rows       int    // >= 1
	Cols       int    // >= 1
}

// GetKey returns the unique identifier for this Collection.
func (c *Collection) GetKey() string {
	return c.Name
}

// SetKey updates the unique identifier for this Collection.
func (c *Collection) SetKey(key string) {
	c.Name = key
}
