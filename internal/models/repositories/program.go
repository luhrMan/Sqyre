package repositories

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/serialize"
	"log"
	"strconv"
	"sync"
)

type ProgramRepository[T models.Program] struct {
	*repository[models.Program]
	// programs map[string]*T
}

var pr *ProgramRepository[models.Program]
var programInit sync.Once

func ProgramRepo() *ProgramRepository[models.Program] {
	programInit.Do(func() {
		pr = &ProgramRepository[models.Program]{
			repository: &repository[models.Program]{
				model:  "programs",
				models: make(map[string]*models.Program),
			},
		}
		pr.models = pr.DecodeAll() //func(s string) { p, _ := pr.Decode(s) })
	})
	return pr
}

func (r *ProgramRepository[T]) New() *models.Program {
	return &models.Program{
		Items: make(map[string]*models.Item),
		Coordinates: map[string]*coordinates.Coordinates{
			strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
				Points:      make(map[string]*coordinates.Point),
				SearchAreas: make(map[string]coordinates.SearchArea),
			},
		},
	}
}

func (r *ProgramRepository[T]) Decode(s string) (*models.Program, error) {
	return r.m.Decode(s)
}

func (r *ProgramRepository[T]) DecodeAll() map[string]*models.Program {
	var (
		ps = make(map[string]*models.Program)
		ss = serialize.GetViper().GetStringMap("programs")
	)
	for s := range ss {
		p, _ := r.Decode(s)
		ps[s] = p
	}
	log.Printf("Successfully decoded all %v: %v", "programs", ps)
	return ps
}
