package models

// Point represents a named screen coordinate used for click and move actions.
// X and Y may be int (literal) or string (variable reference e.g. "${resultX}").
type Point struct {
	Name string
	X    any
	Y    any
}

// GetKey returns the unique identifier for this Point.
func (p *Point) GetKey() string {
	return p.Name
}

// SetKey updates the unique identifier for this Point.
func (p *Point) SetKey(key string) {
	p.Name = key
}

// SearchArea represents a named rectangular region used for image search operations.
// LeftX, TopY, RightX, and BottomY may be int (literal) or string (variable reference e.g. "${leftX}").
type SearchArea struct {
	Name    string
	LeftX   any
	TopY    any
	RightX  any
	BottomY any
}

// GetKey returns the unique identifier for this SearchArea.
func (sa *SearchArea) GetKey() string {
	return sa.Name
}

// SetKey updates the unique identifier for this SearchArea.
func (sa *SearchArea) SetKey(key string) {
	sa.Name = key
}

// Mask represents a named mask used for template matching in image search.
// Shape is "rectangle" or "circle". For rectangle masks, Base and Height
// define the masked region (supports ${variables}). For circle masks,
// Radius defines the unmasked circle (supports ${variables}).
// CenterX and CenterY position the mask center as a percentage of the
// template dimensions (0% = left/top, 50% = center, 100% = right/bottom).
// When an image file is uploaded the mask is image-based and shape fields
// are ignored.
type Mask struct {
	Name    string
	Shape   string // "rectangle" or "circle"
	CenterX string // X center as % of template width  (supports ${variables})
	CenterY string // Y center as % of template height (supports ${variables})
	Base    string // rectangle base  (supports ${variables})
	Height  string // rectangle height (supports ${variables})
	Radius  string // circle radius    (supports ${variables})
}

// GetKey returns the unique identifier for this Mask.
func (m *Mask) GetKey() string {
	return m.Name
}

// SetKey updates the unique identifier for this Mask.
func (m *Mask) SetKey(key string) {
	m.Name = key
}

// Coordinates is a container for Points and SearchAreas, keyed by resolution
type Coordinates struct {
	Points      map[string]*Point
	SearchAreas map[string]*SearchArea
}
