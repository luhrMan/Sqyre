package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
)

type renamableSaveConfig struct {
	entityType string
	oldName    string
	newName    string
	exists     func(name string) bool
	save       func() error
	deleteOld  func(oldName string) error
	onSuccess  func()
}

// saveRenamableEntity handles overwrite confirmation and delete-then-save rename ordering.
func saveRenamableEntity(cfg renamableSaveConfig) {
	apply := func() {
		if cfg.oldName != cfg.newName && cfg.deleteOld != nil {
			if err := cfg.deleteOld(cfg.oldName); err != nil {
				editorRepoErr("delete", cfg.entityType, cfg.oldName, err)
				return
			}
		}
		if err := cfg.save(); err != nil {
			editorRepoErr("save", cfg.entityType, cfg.newName, err)
			return
		}
		if cfg.onSuccess != nil {
			cfg.onSuccess()
		}
	}

	if cfg.oldName != cfg.newName && cfg.exists != nil && cfg.exists(cfg.newName) {
		if shouldConfirmOverwrite(cfg.entityType, cfg.newName, cfg.exists, activeWire.Window, apply) {
			return
		}
	}
	apply()
}

func saveProgramAfterMutation(program *models.Program, programName string) bool {
	if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
		editorRepoErr("save", "program", programName, err)
		return false
	}
	return true
}

func getProgramForEditor(programName string) (*models.Program, bool) {
	return requireProgram(programName)
}
