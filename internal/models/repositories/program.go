package repositories

import (
	"Squire/internal/models/program"
)

type ProgramRepository struct {
	programs map[string]*program.Program
}

var pr *ProgramRepository

func ProgramRepo() *ProgramRepository {
	return pr
}

func (r *ProgramRepository) Init() {
	r = &ProgramRepository{
		programs: make(map[string]*program.Program),
	}
	r.programs = program.DecodeAll()
	pr = r
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
