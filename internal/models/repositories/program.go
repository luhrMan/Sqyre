package repositories

import (
	"Squire/internal/models/program"
	"sync"
)

type ProgramRepository struct {
	programs map[string]*program.Program
}

var pr *ProgramRepository
var programInit sync.Once

func ProgramRepo() *ProgramRepository {
	programInit.Do(func() {
		pr = &ProgramRepository{
			programs: make(map[string]*program.Program),
		}
		pr.programs = program.DecodeAll()
	})
	return pr
}

func (r *ProgramRepository) Get(s string) *program.Program {
	return r.programs[s]
}

func (r *ProgramRepository) GetAll() map[string]*program.Program {
	return r.programs
}

func (r *ProgramRepository) Set() {
}

func (r *ProgramRepository) SetAll() error {
	e := program.EncodeAll(r.GetAll())
	if e != nil {
		return e
	}
	return nil
}
