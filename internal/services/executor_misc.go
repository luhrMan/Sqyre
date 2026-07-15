package services

import (
	"Sqyre/internal/config"
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
	"log"
	"path/filepath"
)

func init() {
	registerActionRunner("setvariable", executeSetVariable)
	registerActionRunner("savevariable", executeSaveVariable)
	registerActionRunner("foreachrow", executeForEachRowAction)
	registerActionRunner("focuswindow", executeFocusWindow)
	registerActionRunner("runmacro", executeRunMacro)
}

func executeSetVariable(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.SetVariable)
	log.Println("Set Variable:", node.String())
	if macro != nil {
		val, err := macropkg.ResolveSetVariableValue(node.Value, macro)
		if err != nil {
			return fmt.Errorf("set variable %s: %w", node.VariableName, err)
		}
		setMacroVariable(macro, node.VariableName, val)
	}
	return nil
}

func executeSaveVariable(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.SaveVariable)
	log.Println("Save Variable:", node.String())
	if macro != nil && macro.Variables != nil {
		val, ok := macro.Variables.Get(node.VariableName)
		if !ok {
			return fmt.Errorf("variable %s not found", node.VariableName)
		}
		valStr := fmt.Sprintf("%v", val)

		if node.Destination == "clipboard" {
			if err := getAutomationBackend().WriteClipboard(valStr); err != nil {
				return fmt.Errorf("failed to write clipboard: %w", err)
			}
		} else {
			filePath := filepath.Join(config.GetVariablesPath(), node.Destination)
			if err := node.SaveToFile(valStr, filePath); err != nil {
				return fmt.Errorf("failed to save variable to file: %w", err)
			}
		}
	}
	return nil
}

func executeForEachRowAction(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.ForEachRow)
	log.Println("For each row:", node.String())
	return executeForEachRow(node, macro)
}

func executeFocusWindow(a actions.ActionInterface, _ *models.Macro) error {
	node := a.(*actions.FocusWindow)
	log.Println("Focus Window:", node.String())
	return RunFocusWindow(node)
}

func executeRunMacro(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.RunMacro)
	log.Println("Run Macro:", node.String())
	if node.MacroName == "" {
		return fmt.Errorf("run macro: macro name not set")
	}
	targetMacro, err := repositories.MacroRepo().Get(node.MacroName)
	if err != nil {
		return fmt.Errorf("run macro: %w", err)
	}
	if targetMacro.Root == nil {
		return fmt.Errorf("run macro: macro %q has no root", node.MacroName)
	}
	targetMacro.InitRuntimeVariables()
	ApplyMonitorBuiltinVariables(targetMacro)
	if macro != nil {
		highlightFill(macro.Name, node.GetUID(), 0)
		defer highlightClear(macro.Name, node.GetUID())
	}
	return executeRunMacroTree(node, targetMacro, macro)
}
