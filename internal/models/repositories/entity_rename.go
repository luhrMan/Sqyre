package repositories

import (
	"Sqyre/internal/models"
	"log"
)

// PropagateProgramEntityRename updates every macro that references the renamed entity.
// Returns the number of macros that were modified and persisted.
func PropagateProgramEntityRename(kind models.ProgramEntityKind, program, oldName, newName string) (int, error) {
	if oldName == newName {
		return 0, nil
	}
	return propagateMacros(func(m *models.Macro) bool {
		return m.RenameProgramEntity(kind, program, oldName, newName)
	})
}

// PropagateProgramRename updates every macro that references entities under the old program name.
// Returns the number of macros that were modified and persisted.
func PropagateProgramRename(oldProgram, newProgram string) (int, error) {
	if oldProgram == newProgram {
		return 0, nil
	}
	return propagateMacros(func(m *models.Macro) bool {
		return m.RenameProgram(oldProgram, newProgram)
	})
}

// PropagateMacroRename updates every macro that calls the renamed macro via Run Macro.
// Returns the number of macros that were modified and persisted.
func PropagateMacroRename(oldName, newName string) (int, error) {
	if oldName == newName {
		return 0, nil
	}
	return propagateMacros(func(m *models.Macro) bool {
		return m.RenameMacroReference(oldName, newName)
	})
}

// PropagateMaskRenameInProgram updates item mask references within program when a mask is renamed.
// Returns the number of items that were modified and persisted.
func PropagateMaskRenameInProgram(programName, oldName, newName string) (int, error) {
	if oldName == newName {
		return 0, nil
	}
	program, err := ProgramRepo().Get(programName)
	if err != nil {
		return 0, err
	}

	updated := 0
	err = BatchSave(func() error {
		for _, item := range program.ItemRepo().GetAll() {
			if item.Mask != oldName {
				continue
			}
			item.Mask = newName
			if serr := program.ItemRepo().Set(item.Name, item); serr != nil {
				return serr
			}
			updated++
		}
		return nil
	})
	if err != nil {
		return 0, err
	}
	if updated > 0 {
		log.Printf("Propagated mask rename %q -> %q to %d item(s) in program %q", oldName, newName, updated, programName)
	}
	return updated, nil
}

func propagateMacros(mutate func(*models.Macro) bool) (int, error) {
	repo := MacroRepo()
	updated := 0
	err := BatchSave(func() error {
		for _, m := range repo.GetAll() {
			if !mutate(m) {
				continue
			}
			if serr := repo.Set(m.Name, m); serr != nil {
				return serr
			}
			updated++
		}
		return nil
	})
	if err != nil {
		return 0, err
	}
	return updated, nil
}
