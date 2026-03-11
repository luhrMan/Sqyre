// program_common.go holds types and interfaces shared by desktop (program.go) and Android (program_android.go) builds.

package models

// ItemRepositoryInterface defines the interface for item data access operations.
type ItemRepositoryInterface interface {
	Get(name string) (*Item, error)
	GetAll() map[string]*Item
	GetAllKeys() []string
	Set(name string, item *Item) error
	Delete(name string) error
	Save() error
	Count() int
	GetAllWithProgramPrefix() map[string]*Item
	GetAllSorted() []string
	New() *Item
}

// PointRepositoryInterface defines the interface for Point data access operations.
type PointRepositoryInterface interface {
	Get(name string) (*Point, error)
	GetAll() map[string]*Point
	GetAllKeys() []string
	Set(name string, point *Point) error
	Delete(name string) error
	Save() error
	Count() int
	New() *Point
}

// SearchAreaRepositoryInterface defines the interface for SearchArea data access operations.
type SearchAreaRepositoryInterface interface {
	Get(name string) (*SearchArea, error)
	GetAll() map[string]*SearchArea
	GetAllKeys() []string
	Set(name string, area *SearchArea) error
	Delete(name string) error
	Save() error
	Count() int
	New() *SearchArea
}

// ItemRepositoryFactory is set by the repositories package.
var ItemRepositoryFactory func(*Program) ItemRepositoryInterface

// PointRepositoryFactory is set by the repositories package.
var PointRepositoryFactory func(*Program, string) PointRepositoryInterface

// SearchAreaRepositoryFactory is set by the repositories package.
var SearchAreaRepositoryFactory func(*Program, string) SearchAreaRepositoryInterface

// Item is shared by both builds.
type Item struct {
	Name     string   `json:"name"`
	GridSize [2]int   `json:"gridSize"`
	Tags     []string `json:"tags"`
	StackMax int      `json:"stackMax"`
	Merchant string   `json:"merchant"`
}

func (i *Item) GetKey() string   { return i.Name }
func (i *Item) SetKey(key string) { i.Name = key }
