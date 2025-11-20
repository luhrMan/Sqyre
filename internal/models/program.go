package models

import (
	"Squire/internal/config"
	"strconv"
	"sync"

	"gocv.io/x/gocv"
)

// ItemRepositoryInterface defines the interface for item data access operations.
// This interface is defined in the models package to avoid circular dependencies.
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
// This interface is defined in the models package to avoid circular dependencies.
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
// This interface is defined in the models package to avoid circular dependencies.
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

// ItemRepositoryFactory is a function type that creates ItemRepository instances.
// This is set by the repositories package to avoid circular dependencies.
var ItemRepositoryFactory func(*Program) ItemRepositoryInterface

// PointRepositoryFactory is a function type that creates PointRepository instances.
// This is set by the repositories package to avoid circular dependencies.
var PointRepositoryFactory func(*Program, string) PointRepositoryInterface

// SearchAreaRepositoryFactory is a function type that creates SearchAreaRepository instances.
// This is set by the repositories package to avoid circular dependencies.
var SearchAreaRepositoryFactory func(*Program, string) SearchAreaRepositoryInterface

type Program struct {
	Name        string
	Items       map[string]*Item
	Coordinates map[string]*Coordinates
	masks       map[string]func(f ...any) *gocv.Mat

	itemRepo        ItemRepositoryInterface                  // Lazy-initialized ItemRepository
	pointRepos      map[string]PointRepositoryInterface      // Lazy-initialized PointRepositories keyed by resolution
	searchAreaRepos map[string]SearchAreaRepositoryInterface // Lazy-initialized SearchAreaRepositories keyed by resolution
	repoMu          sync.Mutex                               // Protects all repository initialization
}

// GetKey returns the unique identifier for this Program.
func (p *Program) GetKey() string {
	return p.Name
}

// SetKey updates the unique identifier for this Program.
func (p *Program) SetKey(key string) {
	p.Name = key
}

type Item struct {
	Name     string   `json:"name"`
	GridSize [2]int   `json:"gridSize"`
	Tags     []string `json:"tags"`
	StackMax int      `json:"stackMax"`
	Merchant string   `json:"merchant"`
}

// GetKey returns the unique identifier for this Item.
func (i *Item) GetKey() string {
	return i.Name
}

// SetKey updates the unique identifier for this Item.
func (i *Item) SetKey(key string) {
	i.Name = key
}

func NewProgram() *Program {
	return &Program{
		Items: make(map[string]*Item),
		Coordinates: map[string]*Coordinates{
			strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
				Points:      make(map[string]*Point),
				SearchAreas: make(map[string]*SearchArea),
			},
		},
		masks: make(map[string]func(f ...any) *gocv.Mat),
	}
}

func (p *Program) GetMasks() map[string]func(f ...any) *gocv.Mat {
	return p.masks
}

// ItemRepo returns an ItemRepository for managing this program's items.
// The repository is lazily initialized on first access and provides
// thread-safe CRUD operations for items within this program.
// Note: This method is named ItemRepo() instead of Items() to avoid
// conflict with the Items field used for serialization.
func (p *Program) ItemRepo() ItemRepositoryInterface {
	p.repoMu.Lock()
	defer p.repoMu.Unlock()

	if p.itemRepo == nil {
		if ItemRepositoryFactory == nil {
			panic("ItemRepositoryFactory not initialized - repositories package not imported")
		}
		p.itemRepo = ItemRepositoryFactory(p)
	}

	return p.itemRepo
}

// PointRepo returns a PointRepository for managing this program's points at the given resolution.
// The repository is lazily initialized on first access and provides
// thread-safe CRUD operations for points within this program at the specified resolution.
func (p *Program) PointRepo(resolutionKey string) PointRepositoryInterface {
	p.repoMu.Lock()
	defer p.repoMu.Unlock()

	if p.pointRepos == nil {
		p.pointRepos = make(map[string]PointRepositoryInterface)
	}

	if p.pointRepos[resolutionKey] == nil {
		if PointRepositoryFactory == nil {
			panic("PointRepositoryFactory not initialized - repositories package not imported")
		}
		p.pointRepos[resolutionKey] = PointRepositoryFactory(p, resolutionKey)
	}

	return p.pointRepos[resolutionKey]
}

// SearchAreaRepo returns a SearchAreaRepository for managing this program's search areas at the given resolution.
// The repository is lazily initialized on first access and provides
// thread-safe CRUD operations for search areas within this program at the specified resolution.
func (p *Program) SearchAreaRepo(resolutionKey string) SearchAreaRepositoryInterface {
	p.repoMu.Lock()
	defer p.repoMu.Unlock()

	if p.searchAreaRepos == nil {
		p.searchAreaRepos = make(map[string]SearchAreaRepositoryInterface)
	}

	if p.searchAreaRepos[resolutionKey] == nil {
		if SearchAreaRepositoryFactory == nil {
			panic("SearchAreaRepositoryFactory not initialized - repositories package not imported")
		}
		p.searchAreaRepos[resolutionKey] = SearchAreaRepositoryFactory(p, resolutionKey)
	}

	return p.searchAreaRepos[resolutionKey]
}
