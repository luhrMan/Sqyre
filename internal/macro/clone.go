package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
	"fmt"
)

// CloneActionMap converts an action (and its subtree) to a map suitable for paste.
func CloneActionMap(action actions.ActionInterface) (map[string]any, error) {
	return serialize.ActionToMap(action)
}

// PasteActionFromMap creates an action from a clipboard map and attaches it under parent.
func PasteActionFromMap(clipboard map[string]any, parent actions.AdvancedActionInterface) (actions.ActionInterface, error) {
	return serialize.ViperSerializer.CreateActionFromMap(clipboard, parent)
}

// CloneMacroRoot deep-copies a macro root loop via serialize round-trip.
func CloneMacroRoot(src *actions.Loop) (*actions.Loop, error) {
	if src == nil {
		return nil, fmt.Errorf("cannot clone macro root")
	}
	rootMap, err := serialize.ActionToMap(src)
	if err != nil {
		return nil, err
	}
	rootAction, err := serialize.ViperSerializer.CreateActionFromMap(rootMap, nil)
	if err != nil {
		return nil, err
	}
	root, ok := rootAction.(*actions.Loop)
	if !ok {
		return nil, fmt.Errorf("macro root is not a loop")
	}
	return root, nil
}

// CloneMacro deep-copies a macro including root, variable declarations, and tags.
func CloneMacro(src *models.Macro) (*models.Macro, error) {
	if src == nil || src.Root == nil {
		return nil, fmt.Errorf("cannot clone macro")
	}
	root, err := CloneMacroRoot(src.Root)
	if err != nil {
		return nil, err
	}
	decls := make([]models.VariableDecl, len(src.VariableDecls))
	copy(decls, src.VariableDecls)
	tags := append([]string(nil), src.Tags...)

	dup := models.NewMacro("", src.GlobalDelay, nil)
	dup.KeyboardDelay = src.KeyboardDelay
	dup.MouseDelay = src.MouseDelay
	dup.Root = root
	dup.VariableDecls = decls
	dup.Tags = tags
	dup.InitRuntimeVariables()
	return dup, nil
}
