package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

type actionRunner func(a actions.ActionInterface, macro *models.Macro) error

var actionRunners = map[string]actionRunner{}

func registerActionRunner(typeName string, fn actionRunner) {
	actionRunners[typeName] = fn
}

func executeAction(a actions.ActionInterface, macro *models.Macro) error {
	if macro != nil && a != nil {
		if macro.Root == nil || a.GetUID() != macro.Root.GetUID() {
			highlightCursor(macro.Name, a.GetUID())
		}
	}
	if a == nil {
		return nil
	}
	if fn, ok := actionRunners[a.GetType()]; ok {
		return fn(a, macro)
	}
	return nil
}
