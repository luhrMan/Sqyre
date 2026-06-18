package serialize

import (
	"Sqyre/internal/models/actions"
	"fmt"
)

// ActionToMap converts an action (and its subtree) to a map suitable for
// CreateActionFromMap, enabling copy/paste by round-tripping through the map.
func ActionToMap(action actions.ActionInterface) (map[string]any, error) {
	if action == nil {
		return nil, fmt.Errorf("action is nil")
	}
	m := make(map[string]any)
	m["type"] = action.GetType()

	switch a := action.(type) {
	case *actions.Loop:
		m["count"] = a.Count
		m["name"] = a.Name
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.Wait:
		m["time"] = a.Time
	case *actions.FindPixel:
		m["name"] = a.Name
		m["searcharea"] = searchAreaToMap(a.SearchArea)
		m["targetcolor"] = a.TargetColor
		m["colortolerance"] = a.ColorTolerance
		if a.OutputXVariable != "" {
			m["outputxvariable"] = a.OutputXVariable
		}
		if a.OutputYVariable != "" {
			m["outputyvariable"] = a.OutputYVariable
		}
		if a.WaitTilFound {
			m["waittilfound"] = a.WaitTilFound
			m["waittilfoundseconds"] = a.WaitTilFoundSeconds
			if a.WaitTilFoundIntervalMs > 0 {
				m["waittilfoundintervalms"] = a.WaitTilFoundIntervalMs
			}
		}
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.Click:
		m["button"] = a.Button
		m["state"] = a.State
	case *actions.Move:
		m["point"] = pointToMap(a.Point)
		m["smooth"] = a.Smooth
	case *actions.Key:
		m["key"] = a.Key
		m["state"] = a.State
	case *actions.Type:
		m["text"] = a.Text
		m["delayms"] = a.DelayMs
	case *actions.ImageSearch:
		m["name"] = a.Name
		m["targets"] = a.Targets
		m["searcharea"] = searchAreaToMap(a.SearchArea)
		m["rowsplit"] = a.RowSplit
		m["colsplit"] = a.ColSplit
		m["tolerance"] = float64(a.Tolerance)
		m["blur"] = a.Blur
		if a.OutputXVariable != "" {
			m["outputxvariable"] = a.OutputXVariable
		}
		if a.OutputYVariable != "" {
			m["outputyvariable"] = a.OutputYVariable
		}
		if a.WaitTilFound {
			m["waittilfound"] = a.WaitTilFound
			m["waittilfoundseconds"] = a.WaitTilFoundSeconds
			if a.WaitTilFoundIntervalMs > 0 {
				m["waittilfoundintervalms"] = a.WaitTilFoundIntervalMs
			}
		}
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.Ocr:
		m["name"] = a.Name
		m["target"] = a.Target
		m["searcharea"] = searchAreaToMap(a.SearchArea)
		if a.OutputVariable != "" {
			m["outputvariable"] = a.OutputVariable
		}
		if a.OutputXVariable != "" {
			m["outputxvariable"] = a.OutputXVariable
		}
		if a.OutputYVariable != "" {
			m["outputyvariable"] = a.OutputYVariable
		}
		if a.WaitTilFound {
			m["waittilfound"] = a.WaitTilFound
			m["waittilfoundseconds"] = a.WaitTilFoundSeconds
			if a.WaitTilFoundIntervalMs > 0 {
				m["waittilfoundintervalms"] = a.WaitTilFoundIntervalMs
			}
		}
		if !a.Grayscale {
			m["grayscale"] = a.Grayscale
		}
		if a.Blur != 0 {
			m["blur"] = a.Blur
		}
		if a.MinThreshold != 0 {
			m["minthreshold"] = a.MinThreshold
		}
		if a.Resize != 1.0 {
			m["resize"] = a.Resize
		}
		if a.ThresholdOtsu {
			m["thresholdotsu"] = a.ThresholdOtsu
		}
		if a.ThresholdInvert {
			m["thresholdinvert"] = a.ThresholdInvert
		}
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.SetVariable:
		m["variablename"] = a.VariableName
		m["value"] = a.Value
	case *actions.Calculate:
		m["expression"] = a.Expression
		m["outputvar"] = a.OutputVar
	case *actions.Conditional:
		m["name"] = a.Name
		m["operator"] = a.Operator
		m["left"] = a.Left
		m["right"] = a.Right
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.ForEachRow:
		m["name"] = a.Name
		m["sources"] = listColumnsToMaps(a.Sources)
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.SaveVariable:
		m["variablename"] = a.VariableName
		m["destination"] = a.Destination
		m["append"] = a.Append
		m["appendnewline"] = a.AppendNewline
	// case *actions.Calibration:
	// 	m["name"] = a.Name
	// 	m["programname"] = a.ProgramName
	// 	if a.ResolutionKey != "" {
	// 		m["resolutionkey"] = a.ResolutionKey
	// 	}
	// 	m["searcharea"] = searchAreaToMap(a.SearchArea)
	// 	m["targets"] = calibrationTargetsToMaps(a.Targets)
	// 	m["rowsplit"] = a.RowSplit
	// 	m["colsplit"] = a.ColSplit
	// 	m["tolerance"] = float64(a.Tolerance)
	// 	m["blur"] = a.Blur
	case *actions.FocusWindow:
		m["windowtarget"] = a.WindowTarget
	case *actions.RunMacro:
		m["macroname"] = a.MacroName
	default:
		return nil, fmt.Errorf("unknown action type: %T", action)
	}
	return m, nil
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

func pointToMap(p actions.Point) map[string]any {
	return map[string]any{
		"name": p.Name,
		"x":    p.X,
		"y":    p.Y,
	}
}

func searchAreaToMap(s actions.SearchArea) map[string]any {
	return map[string]any{
		"name":    s.Name,
		"leftx":   s.LeftX,
		"topy":    s.TopY,
		"rightx":  s.RightX,
		"bottomy": s.BottomY,
	}
}

// func calibrationTargetsToMaps(t []actions.CalibrationTarget) []any {
// 	out := make([]any, 0, len(t))
// 	for _, c := range t {
// 		out = append(out, map[string]any{
// 			"outputname": c.OutputName,
// 			"outputtype": c.OutputType,
// 			"target":     c.Target,
// 		})
// 	}
// 	return out
// }
