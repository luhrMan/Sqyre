package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"errors"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

type nestedCreateOpts struct {
	objectType   string
	programName  func(map[string]fyne.CanvasObject) string
	ensureUnique func(*models.Program, string) error
	build        func(*models.Program, map[string]fyne.CanvasObject) (any, error)
	applyUI      func(any, string)
	refresh      func()
}

func saveNestedEntity(opts nestedCreateOpts, w map[string]fyne.CanvasObject) error {
	nameEntry, ok := w["Name"].(*widget.Entry)
	if !ok {
		return errors.New("name field missing")
	}
	name := nameEntry.Text
	if err := validateCreateName(name); err != nil {
		return err
	}
	programName := opts.programName(w)
	if err := validateCreateProgramName(programName); err != nil {
		return err
	}
	pro := getOrCreateProgram(programName)
	if pro == nil {
		return errors.New("failed to get or create program")
	}
	if err := opts.ensureUnique(pro, name); err != nil {
		return err
	}
	entity, err := opts.build(pro, w)
	if err != nil {
		return err
	}
	opts.applyUI(entity, programName)
	return nil
}

func ensureUniqueInRepo(objectType string, get func(string) (any, error)) func(*models.Program, string) error {
	return func(_ *models.Program, name string) error {
		return ensureNameAvailable(name, objectType, get)
	}
}

func programRepoGet(name string) (any, error) {
	return repositories.ProgramRepo().Get(name)
}
