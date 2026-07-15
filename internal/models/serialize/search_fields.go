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
	if v, ok := raw["repeatmode"].(string); ok {
		out.RepeatMode = v
	}
	if out.RepeatMode == "" {
		out.RepeatMode = actions.RepeatOnce
	}
	if v := raw["waittilfoundseconds"]; v != nil {
		out.WaitTilFoundSeconds = intFromMap(v)
	}
	if v := raw["waittilfoundintervalms"]; v != nil {
		out.WaitTilFoundIntervalMs = intFromMap(v)
	}
	if v := raw["maxiterations"]; v != nil {
		out.MaxIterations = intFromMap(v)
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
	mode := w.EffectiveRepeatMode()
	m["repeatmode"] = mode
	if mode == actions.RepeatWaitUntilFound {
		m["waittilfoundseconds"] = w.WaitTilFoundSeconds
	} else if w.WaitTilFoundSeconds > 0 {
		m["waittilfoundseconds"] = w.WaitTilFoundSeconds
	}
	if w.WaitTilFoundIntervalMs > 0 {
		m["waittilfoundintervalms"] = w.WaitTilFoundIntervalMs
	}
	if mode == actions.RepeatWhileFound {
		m["maxiterations"] = w.EffectiveMaxIterations()
	}
}
