package serialize

import "Sqyre/internal/models/actions"

// ActionToMap converts an action (and its subtree) to a map suitable for
// CreateActionFromMap, enabling copy/paste by round-tripping through the map.
func ActionToMap(action actions.ActionInterface) (map[string]any, error) {
	return encodeActionToMap(action)
}

func subActionsToMaps(sa []actions.ActionInterface) ([]any, error) {
	out := make([]any, 0, len(sa))
	for _, sub := range sa {
		sm, err := ActionToMap(sub)
		if err != nil {
			return nil, err
		}
		out = append(out, sm)
	}
	return out, nil
}

func coordinateRefToMap(r actions.CoordinateRef) any {
	if r.IsEmpty() {
		return nil
	}
	return string(r)
}
