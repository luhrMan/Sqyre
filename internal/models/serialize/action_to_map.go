package serialize

import (
	"Squire/internal/models/actions"
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
	case *actions.WaitForPixel:
		m["name"] = a.Name
		m["point"] = pointToMap(a.Point)
		m["targetcolor"] = a.TargetColor
		m["colortolerance"] = a.ColorTolerance
		m["timeoutseconds"] = a.TimeoutSeconds
		subs, err := subActionsToMaps(a.GetSubActions())
		if err != nil {
			return nil, err
		}
		m["subactions"] = subs
	case *actions.Click:
		m["button"] = a.Button
		m["hold"] = a.Hold
	case *actions.Move:
		m["point"] = pointToMap(a.Point)
	case *actions.Key:
		m["key"] = a.Key
		m["state"] = a.State
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
		if a.WaitTilFound {
			m["waittilfound"] = a.WaitTilFound
			m["waittilfoundseconds"] = a.WaitTilFoundSeconds
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
	case *actions.DataList:
		m["source"] = a.Source
		m["outputvar"] = a.OutputVar
		m["lengthvar"] = a.LengthVar
		m["isfile"] = a.IsFile
		m["skipblanklines"] = a.SkipBlankLines
	case *actions.SaveVariable:
		m["variablename"] = a.VariableName
		m["destination"] = a.Destination
		m["append"] = a.Append
		m["appendnewline"] = a.AppendNewline
	case *actions.Calibration:
		m["name"] = a.Name
		m["programname"] = a.ProgramName
		if a.ResolutionKey != "" {
			m["resolutionkey"] = a.ResolutionKey
		}
		m["searcharea"] = searchAreaToMap(a.SearchArea)
		m["targets"] = calibrationTargetsToMaps(a.Targets)
		m["rowsplit"] = a.RowSplit
		m["colsplit"] = a.ColSplit
		m["tolerance"] = float64(a.Tolerance)
		m["blur"] = a.Blur
	case *actions.FocusWindow:
		m["windowtarget"] = a.WindowTarget
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

func calibrationTargetsToMaps(t []actions.CalibrationTarget) []any {
	out := make([]any, 0, len(t))
	for _, c := range t {
		out = append(out, map[string]any{
			"outputname": c.OutputName,
			"outputtype": c.OutputType,
			"target":     c.Target,
		})
	}
	return out
}
