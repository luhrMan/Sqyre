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
// The delete and save are coalesced into a single disk write via BatchSave so a
// rename persists the (whole) parent aggregate once instead of twice.
func saveRenamableEntity(cfg renamableSaveConfig) {
	apply := func() {
		err := repositories.BatchSave(func() error {
			if cfg.oldName != cfg.newName && cfg.deleteOld != nil {
				if derr := cfg.deleteOld(cfg.oldName); derr != nil {
					editorRepoErr("delete", cfg.entityType, cfg.oldName, derr)
					return derr
				}
			}
			if serr := cfg.save(); serr != nil {
				editorRepoErr("save", cfg.entityType, cfg.newName, serr)
				return serr
			}
			return nil
		})
		if err != nil {
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

func getProgramForEditor(programName string) (*models.Program, bool) {
	return requireProgram(programName)
}
