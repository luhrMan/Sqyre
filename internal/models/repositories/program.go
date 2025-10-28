package repositories

import (
	"Squire/internal/models/coordinates"
	"Squire/internal/models/items"
	"Squire/internal/models/program"
	"Squire/internal/models/serialize"
	"log"
	"sync"
)

type ProgramRepository[T program.Program] struct {
	*repository[T]
	// programs map[string]*T
}

var pr *ProgramRepository[program.Program]
var programInit sync.Once

func ProgramRepo() *ProgramRepository[program.Program] {
	programInit.Do(func() {
		pr = &ProgramRepository[program.Program]{
			repository: &repository[program.Program]{
				model:  "programs",
				models: make(map[string]*program.Program),
			},
		}
		pr.models = pr.DecodeAll(pr.model, pr.Decode) //func(s string) { p, _ := pr.Decode(s) })
	})
	return pr
}

func (r *ProgramRepository[T]) Decode(s string) (*program.Program, error) {
	var (
		keyStr = "programs" + "." + s + "."
		err    error
		errStr = "problem here lol"
	)

	var p = &program.Program{
		Items:       map[string]*items.Item{},
		Coordinates: map[string]*coordinates.Coordinates{},
	}
	err = serialize.GetViper().UnmarshalKey(keyStr+"name", &p.Name)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = serialize.GetViper().UnmarshalKey(keyStr+"items", &p.Items)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = serialize.GetViper().UnmarshalKey(keyStr+"coordinates", &p.Coordinates)
	if err != nil {
		log.Fatalf(errStr, err)
	}
	log.Println("Successfully decoded program:", p.Name)
	return p, nil
}
