package services

import (
	"errors"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func isKeyboardAction(a actions.ActionInterface) bool {
	if a == nil {
		return false
	}
	switch a.GetType() {
	case "key", "type":
		return true
	default:
		return false
	}
}

func isMouseAction(a actions.ActionInterface) bool {
	if a == nil {
		return false
	}
	switch a.GetType() {
	case "move", "click":
		return true
	default:
		return false
	}
}

// applyActionDelay sleeps for the macro's configured delays (ms) after an action completes.
// Global delay applies after every action; keyboard and mouse delays add extra time for
// their respective action categories.
func applyActionDelay(macro *models.Macro, a actions.ActionInterface) error {
	if macro == nil {
		return nil
	}
	if macro.GlobalDelay > 0 {
		if err := interruptibleSleep(macro.GlobalDelay); err != nil {
			return err
		}
	}
	if isKeyboardAction(a) && macro.KeyboardDelay > 0 {
		if err := interruptibleSleep(macro.KeyboardDelay); err != nil {
			return err
		}
	}
	if isMouseAction(a) && macro.MouseDelay > 0 {
		if err := interruptibleSleep(macro.MouseDelay); err != nil {
			return err
		}
	}
	return nil
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
