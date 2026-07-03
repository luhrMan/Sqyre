package macro

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
	"fmt"
)

// SnapshotRoot captures the macro tree and selected UID for undo/redo.
func SnapshotRoot(root *actions.Loop, selectedUID string) (rootMap map[string]any, selected string, err error) {
	if root == nil {
		return nil, selectedUID, nil
	}
	rootMap, err = serialize.ActionToMap(root)
	if err != nil {
		return nil, "", err
	}
	injectActionUID(rootMap, root)
	return rootMap, selectedUID, nil
}

// RestoreRoot rebuilds a macro root loop from a snapshot map.
func RestoreRoot(rootMap map[string]any) (*actions.Loop, error) {
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

// injectActionUID stores action UIDs in snapshot maps so undo/redo restores
// stable identities without affecting copy/paste (which omits uid).
func injectActionUID(m map[string]any, action actions.ActionInterface) {
	if uid := action.GetUID(); uid != "" {
		m["uid"] = uid
	}
	adv, ok := action.(actions.AdvancedActionInterface)
	if !ok {
		return
	}
	subsRaw, ok := m["subactions"].([]any)
	if !ok {
		return
	}
	subs := adv.GetSubActions()
	for i := 0; i < len(subs) && i < len(subsRaw); i++ {
		subMap, ok := subsRaw[i].(map[string]any)
		if !ok {
			continue
		}
		injectActionUID(subMap, subs[i])
	}
}
