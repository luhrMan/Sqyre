package macro

import (
	macrologic "Sqyre/internal/macro"
	"log"

	"Sqyre/internal/models/repositories"
)

// copiedNodeMap is the clipboard for macro tree copy/paste (nil when empty).
var copiedNodeMap map[string]any

func copyMacroTreeSelection(mt *MacroTree) bool {
	if mt == nil || mt.SelectedNode == "" {
		return false
	}
	node := mt.Macro.Root.GetAction(mt.SelectedNode)
	if node == nil || node.GetParent() == nil {
		return false
	}
	m, err := macrologic.CloneActionMap(node)
	if err != nil {
		return false
	}
	copiedNodeMap = m
	return true
}

func pasteMacroTreeClipboard(mt *MacroTree) bool {
	if mt == nil {
		return false
	}
	if !mt.PasteNode(copiedNodeMap) {
		return false
	}
	if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
		log.Printf("failed to save macro after paste: %v", err)
		return false
	}
	return true
}

func canCopyMacroTreeSelection(mt *MacroTree) bool {
	if mt == nil || mt.SelectedNode == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return false
	}
	node := mt.Macro.Root.GetAction(mt.SelectedNode)
	return node != nil && node.GetParent() != nil
}

func canPasteMacroTreeClipboard(mt *MacroTree) bool {
	if mt == nil || copiedNodeMap == nil {
		return false
	}
	_, _, ok := mt.insertLocationBelowSelection()
	return ok
}

// PasteNode creates a copy of the action from clipboardMap and inserts it using
// the same placement rules as InsertActionBelowSelection. Returns true if paste
// succeeded.
func (mt *MacroTree) PasteNode(clipboardMap map[string]any) bool {
	if clipboardMap == nil {
		return false
	}
	parent, insertIndex, ok := mt.insertLocationBelowSelection()
	if !ok {
		return false
	}
	newAction, err := macrologic.PasteActionFromMap(clipboardMap, parent)
	if err != nil {
		return false
	}
	mt.recordMutation()
	mt.insertActionAt(parent, insertIndex, newAction)
	mt.Select(newAction.GetUID())
	mt.SelectedNode = newAction.GetUID()
	mt.Refresh()
	return true
}
