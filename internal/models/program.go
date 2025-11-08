package models

import (
	"Squire/internal/config"
	"Squire/internal/models/coordinates"
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
}

// ItemRepositoryFactory is a function type that creates ItemRepository instances.
// This is set by the repositories package to avoid circular dependencies.
var ItemRepositoryFactory func(*Program) ItemRepositoryInterface

type Program struct {
	Name        string
	Items       map[string]*Item
	Coordinates map[string]*coordinates.Coordinates
	masks       map[string]func(f ...any) *gocv.Mat
	
	itemRepo   ItemRepositoryInterface // Lazy-initialized ItemRepository
	itemRepoMu sync.Mutex              // Protects itemRepo initialization
}

type Item struct {
	Name     string   `json:"name"`
	GridSize [2]int   `json:"gridSize"`
	Tags     []string `json:"tags"`
	StackMax int      `json:"stackMax"`
	Merchant string   `json:"merchant"`
}

func NewProgram() *Program {
	return &Program{
		Items: make(map[string]*Item),
		Coordinates: map[string]*coordinates.Coordinates{
			strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
				Points:      make(map[string]*coordinates.Point),
				SearchAreas: make(map[string]*coordinates.SearchArea),
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
	p.itemRepoMu.Lock()
	defer p.itemRepoMu.Unlock()
	
	if p.itemRepo == nil {
		if ItemRepositoryFactory == nil {
			panic("ItemRepositoryFactory not initialized - repositories package not imported")
		}
		p.itemRepo = ItemRepositoryFactory(p)
	}
	
	return p.itemRepo
}