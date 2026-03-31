//go:build !js

package appdata

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
)

func init() {
	Register(repoPrograms{}, repoMacros{})
}

type repoPrograms struct{}

func (repoPrograms) Get(name string) (*models.Program, error) {
	return repositories.ProgramRepo().Get(name)
}

func (repoPrograms) Set(name string, p *models.Program) error {
	return repositories.ProgramRepo().Set(name, p)
}

func (repoPrograms) Delete(name string) error {
	return repositories.ProgramRepo().Delete(name)
}

func (repoPrograms) GetAll() map[string]*models.Program {
	return repositories.ProgramRepo().GetAll()
}

func (repoPrograms) GetAllKeys() []string {
	return repositories.ProgramRepo().GetAllKeys()
}

func (repoPrograms) New() *models.Program {
	return repositories.ProgramRepo().New()
}

func (repoPrograms) GetAllSortedByName() []*models.Program {
	return repositories.ProgramRepo().GetAllSortedByName()
}

type repoMacros struct{}

func (repoMacros) Get(name string) (*models.Macro, error) {
	return repositories.MacroRepo().Get(name)
}

func (repoMacros) Set(name string, m *models.Macro) error {
	return repositories.MacroRepo().Set(name, m)
}

func (repoMacros) Delete(name string) error {
	return repositories.MacroRepo().Delete(name)
}

func (repoMacros) GetAll() map[string]*models.Macro {
	return repositories.MacroRepo().GetAll()
}

func (repoMacros) GetAllKeys() []string {
	return repositories.MacroRepo().GetAllKeys()
}

func (repoMacros) New() *models.Macro {
	return repositories.MacroRepo().New()
}
