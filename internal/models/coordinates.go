package models

// Point represents a named screen coordinate used for click and move actions.
// X and Y may be int (literal) or string (variable reference e.g. "${resultX}").
type Point struct {
	Name string
	X    interface{}
	Y    interface{}
}

// GetKey returns the unique identifier for this Point.
func (p *Point) GetKey() string {
	return p.Name
}

// SetKey updates the unique identifier for this Point.
func (p *Point) SetKey(key string) {
	p.Name = key
}

// SearchArea represents a named rectangular region used for image search operations
type SearchArea struct {
	Name    string
	LeftX   int
	TopY    int
	RightX  int
	BottomY int
}

// GetKey returns the unique identifier for this SearchArea.
func (sa *SearchArea) GetKey() string {
	return sa.Name
}

// SetKey updates the unique identifier for this SearchArea.
func (sa *SearchArea) SetKey(key string) {
	sa.Name = key
}

// Coordinates is a container for Points and SearchAreas, keyed by resolution
type Coordinates struct {
	Points      map[string]*Point
	SearchAreas map[string]*SearchArea
}
