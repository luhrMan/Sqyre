//go:build android

package models

import (
	"Squire/internal/config"
	"strconv"
	"sync"
)

type Program struct {
	Name        string
	Items       map[string]*Item
	Coordinates map[string]*Coordinates
	masks       map[string]func(f ...any) interface{} // stub on Android (no gocv)

	itemRepo        ItemRepositoryInterface
	pointRepos      map[string]PointRepositoryInterface
	searchAreaRepos map[string]SearchAreaRepositoryInterface
	repoMu          sync.Mutex
}

func (p *Program) GetKey() string    { return p.Name }
func (p *Program) SetKey(key string) { p.Name = key }

func NewProgram() *Program {
	return &Program{
		Items: make(map[string]*Item),
		Coordinates: map[string]*Coordinates{
			strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): {
				Points:      make(map[string]*Point),
				SearchAreas: make(map[string]*SearchArea),
			},
		},
		masks: make(map[string]func(f ...any) interface{}),
	}
}

func (p *Program) GetMasks() map[string]func(f ...any) interface{} {
	return p.masks
}

func (p *Program) ItemRepo() ItemRepositoryInterface {
	p.repoMu.Lock()
	defer p.repoMu.Unlock()
	if p.itemRepo == nil {
		if ItemRepositoryFactory == nil {
			panic("ItemRepositoryFactory not initialized")
		}
		p.itemRepo = ItemRepositoryFactory(p)
	}
	return p.itemRepo
}

func (p *Program) PointRepo(resolutionKey string) PointRepositoryInterface {
	p.repoMu.Lock()
	defer p.repoMu.Unlock()
	if p.pointRepos == nil {
		p.pointRepos = make(map[string]PointRepositoryInterface)
	}
	if p.pointRepos[resolutionKey] == nil {
		if PointRepositoryFactory == nil {
			panic("PointRepositoryFactory not initialized")
		}
		p.pointRepos[resolutionKey] = PointRepositoryFactory(p, resolutionKey)
	}
	return p.pointRepos[resolutionKey]
}

func (p *Program) SearchAreaRepo(resolutionKey string) SearchAreaRepositoryInterface {
	p.repoMu.Lock()
	defer p.repoMu.Unlock()
	if p.searchAreaRepos == nil {
		p.searchAreaRepos = make(map[string]SearchAreaRepositoryInterface)
	}
	if p.searchAreaRepos[resolutionKey] == nil {
		if SearchAreaRepositoryFactory == nil {
			panic("SearchAreaRepositoryFactory not initialized")
		}
		p.searchAreaRepos[resolutionKey] = SearchAreaRepositoryFactory(p, resolutionKey)
	}
	return p.searchAreaRepos[resolutionKey]
}
