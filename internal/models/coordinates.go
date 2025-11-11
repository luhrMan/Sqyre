package models

import (
	"errors"
	"slices"
	"strings"
)

// Point represents a named screen coordinate used for click and move actions
type Point struct {
	Name string
	X    int
	Y    int
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

// Legacy methods for backward compatibility - these will be removed in task 8
func (c *Coordinates) GetPoint(s string) (*Point, error) {
	if p, ok := c.Points[strings.ToLower(s)]; ok {
		return p, nil
	}
	return nil, errors.New("Could not get point " + s)
}

func (c *Coordinates) GetPoints() map[string]*Point {
	return c.Points
}

func (c *Coordinates) GetPointsAsStringSlice() []string {
	keys := make([]string, len(c.Points))
	i := 0
	for _, p := range c.Points {
		keys[i] = p.Name
		i++
	}
	slices.Sort(keys)
	return keys
}

func (c *Coordinates) SetPoint(p *Point) {
	c.Points[strings.ToLower(p.Name)] = p
}

func (c *Coordinates) DeletePoint(name string) {
	delete(c.Points, strings.ToLower(name))
}

func (c *Coordinates) AddPoint(p Point) (*Point, error) {
	if _, ok := c.Points[strings.ToLower(p.Name)]; ok {
		return nil, errors.New("a point with that name already exists")
	}
	c.SetPoint(&p)
	return &p, nil
}

func (c *Coordinates) GetSearchArea(s string) (*SearchArea, error) {
	if sa, ok := c.SearchAreas[strings.ToLower(s)]; ok {
		return sa, nil
	}
	return nil, errors.New("Could not get search area " + s)
}

func (c *Coordinates) GetSearchAreas() map[string]*SearchArea {
	return c.SearchAreas
}

func (c *Coordinates) GetSearchAreasAsStringSlice() []string {
	keys := make([]string, len(c.SearchAreas))
	i := 0
	for k := range c.SearchAreas {
		keys[i] = k
		i++
	}
	return keys
}

func (c *Coordinates) SetSearchArea(sa *SearchArea) {
	c.SearchAreas[sa.Name] = sa
}

func (c *Coordinates) AddSearchArea(sa SearchArea) (*SearchArea, error) {
	if _, ok := c.SearchAreas[strings.ToLower(sa.Name)]; ok {
		return nil, errors.New("a search area with that name already exists")
	}
	c.SetSearchArea(&sa)
	return &sa, nil
}

func (c *Coordinates) DeleteSearchArea(name string) {
	delete(c.SearchAreas, strings.ToLower(name))
}
