package editor

import (
	"fmt"
	"log"

	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
)

func editorErr(err error) {
	if err == nil {
		return
	}
	activeWire.ShowErrorWithEscape(err, activeWire.Window)
}

func editorRepoErr(op, name string, err error) {
	if err == nil {
		return
	}
	editorErr(fmt.Errorf("%s %q: %w", op, name, err))
}

func editorRepoLog(op, entity, name string, err error) {
	if err != nil {
		log.Printf("editor %s %s %q: %v", op, entity, name, err)
	}
}

func requireProgram(programName string) (*models.Program, bool) {
	if programName == "" {
		editorErr(fmt.Errorf("program cannot be empty"))
		return nil, false
	}
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil {
		editorRepoErr("load", programName, err)
		return nil, false
	}
	return program, true
}
