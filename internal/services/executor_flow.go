package services

import (
	"errors"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

// applyGlobalDelay sleeps for the macro's GlobalDelay (ms) after an action completes.
func applyGlobalDelay(macro *models.Macro) error {
	if macro == nil || macro.GlobalDelay <= 0 {
		return nil
	}
	return interruptibleSleep(macro.GlobalDelay)
}

func executeSubActions(subs []actions.ActionInterface, macro *models.Macro) error {
	for _, action := range subs {
		if err := checkMacroStop(); err != nil {
			return err
		}
		if err := executeWithContext(action, macro); err != nil {
			return err
		}
	}
	return nil
}

func handleLoopFlow(err error) (breakLoop, continueLoop bool, fatal error) {
	if err == nil {
		return false, false, nil
	}
	if errors.Is(err, actions.ErrContinue) {
		return false, true, nil
	}
	if errors.Is(err, actions.ErrBreak) {
		return true, false, nil
	}
	return false, false, err
}
