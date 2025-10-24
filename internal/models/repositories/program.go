package repositories

import (
	"Squire/internal/models/program"
)

var programs map[string]*program.Program

func InitPrograms() {
	programs = program.GetPrograms()
}

func GetProgram(s string) *program.Program {
	return programs[s]
}

func GetPrograms() map[string]*program.Program {
	return programs
}
