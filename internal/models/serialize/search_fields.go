package serialize

import "Sqyre/internal/models/actions"

func decodeCoordinateOutputs(raw map[string]any, out *actions.CoordinateOutputs) {
	if v, ok := raw["outputxvariable"].(string); ok {
		out.OutputXVariable = v
	}
	if v, ok := raw["outputyvariable"].(string); ok {
		out.OutputYVariable = v
	}
}

func decodeWaitTilFound(raw map[string]any, out *actions.WaitTilFoundConfig) {
	if v, ok := raw["waittilfound"].(bool); ok {
		out.WaitTilFound = v
	}
	if v := raw["waittilfoundseconds"]; v != nil {
		out.WaitTilFoundSeconds = intFromMap(v)
	}
	if v := raw["waittilfoundintervalms"]; v != nil {
		out.WaitTilFoundIntervalMs = intFromMap(v)
	}
}

func writeCoordinateOutputs(m map[string]any, out actions.CoordinateOutputs) {
	if out.OutputXVariable != "" {
		m["outputxvariable"] = out.OutputXVariable
	}
	if out.OutputYVariable != "" {
		m["outputyvariable"] = out.OutputYVariable
	}
}

func writeWaitTilFound(m map[string]any, w actions.WaitTilFoundConfig) {
	if !w.WaitTilFound {
		return
	}
	m["waittilfound"] = w.WaitTilFound
	m["waittilfoundseconds"] = w.WaitTilFoundSeconds
	if w.WaitTilFoundIntervalMs > 0 {
		m["waittilfoundintervalms"] = w.WaitTilFoundIntervalMs
	}
}
