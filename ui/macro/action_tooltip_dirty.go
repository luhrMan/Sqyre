package macro

import (
	"bytes"
	"encoding/json"
	"slices"

	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
)

func snapshotActionMap(action actions.ActionInterface) (map[string]any, error) {
	if action == nil {
		return nil, nil
	}
	return serialize.ActionToMap(action)
}

func actionMapsEqual(a, b map[string]any) bool {
	if a == nil && b == nil {
		return true
	}
	if a == nil || b == nil {
		return false
	}
	ja, errA := json.Marshal(a)
	jb, errB := json.Marshal(b)
	if errA != nil || errB != nil {
		return false
	}
	return bytes.Equal(ja, jb)
}

func (form *tooltipEditForm) hasPendingChanges(node actions.ActionInterface) bool {
	if form.baseline == nil || form.applyAction == nil {
		return false
	}
	if err := form.applyAction(); err != nil {
		_ = restoreActionFromBaseline(node, form.baseline)
		return false
	}
	current, err := serialize.ActionToMap(node)
	if err != nil {
		_ = restoreActionFromBaseline(node, form.baseline)
		return false
	}
	dirty := !actionMapsEqual(form.baseline, current)
	_ = restoreActionFromBaseline(node, form.baseline)
	return dirty
}

func restoreActionFromBaseline(node actions.ActionInterface, baseline map[string]any) error {
	if node == nil || baseline == nil {
		return nil
	}
	restored, err := serialize.ViperSerializer.CreateActionFromMap(baseline, nil)
	if err != nil {
		return err
	}
	copyActionFromSnapshot(node, restored)
	return nil
}

func copyActionFromSnapshot(dst, src actions.ActionInterface) {
	if dst == nil || src == nil || dst.GetType() != src.GetType() {
		return
	}
	uid := dst.GetUID()
	switch d := dst.(type) {
	case *actions.Move:
		s := src.(*actions.Move)
		d.Point = s.Point
		d.Smooth = s.Smooth
		d.SmoothLow = s.SmoothLow
		d.SmoothHigh = s.SmoothHigh
		d.SmoothDelayMs = s.SmoothDelayMs
	case *actions.Click:
		s := src.(*actions.Click)
		d.Button = s.Button
		d.State = s.State
	case *actions.Key:
		s := src.(*actions.Key)
		d.Key = s.Key
		d.State = s.State
	case *actions.Wait:
		s := src.(*actions.Wait)
		d.Time = s.Time
	case *actions.Loop:
		s := src.(*actions.Loop)
		d.Name = s.Name
		d.Count = s.Count
	case *actions.Conditional:
		s := src.(*actions.Conditional)
		d.Name = s.Name
		d.Match = s.Match
		d.Clauses = slices.Clone(s.Clauses)
	case *actions.SetVariable:
		s := src.(*actions.SetVariable)
		d.VariableName = s.VariableName
		d.Value = s.Value
	case *actions.Calculate:
		s := src.(*actions.Calculate)
		d.Expression = s.Expression
		d.OutputVar = s.OutputVar
	case *actions.RunMacro:
		s := src.(*actions.RunMacro)
		d.MacroName = s.MacroName
	case *actions.ImageSearch:
		s := src.(*actions.ImageSearch)
		d.Name = s.Name
		d.Targets = slices.Clone(s.Targets)
		d.SearchArea = s.SearchArea
		d.RowSplit = s.RowSplit
		d.ColSplit = s.ColSplit
		d.Tolerance = s.Tolerance
		d.Blur = s.Blur
		d.WaitTilFoundConfig = s.WaitTilFoundConfig
		d.CoordinateOutputs = s.CoordinateOutputs
		d.RunBranchOnNoFind = s.RunBranchOnNoFind
	case *actions.FindPixel:
		s := src.(*actions.FindPixel)
		d.Name = s.Name
		d.SearchArea = s.SearchArea
		d.TargetColor = s.TargetColor
		d.ColorTolerance = s.ColorTolerance
		d.WaitTilFoundConfig = s.WaitTilFoundConfig
		d.CoordinateOutputs = s.CoordinateOutputs
	case *actions.Ocr:
		s := src.(*actions.Ocr)
		d.Name = s.Name
		d.Target = s.Target
		d.SearchArea = s.SearchArea
		d.WaitTilFoundConfig = s.WaitTilFoundConfig
		d.CoordinateOutputs = s.CoordinateOutputs
	case *actions.SemanticSearch:
		s := src.(*actions.SemanticSearch)
		d.Name = s.Name
		d.Prompt = s.Prompt
		d.SearchArea = s.SearchArea
		d.ConfidenceThreshold = s.ConfidenceThreshold
		d.IoUThreshold = s.IoUThreshold
		d.MaxMatches = s.MaxMatches
		d.OutputLabelVariable = s.OutputLabelVariable
		d.WaitTilFoundConfig = s.WaitTilFoundConfig
		d.CoordinateOutputs = s.CoordinateOutputs
		d.RunBranchOnNoFind = s.RunBranchOnNoFind
	case *actions.Type:
		s := src.(*actions.Type)
		d.Text = s.Text
		d.DelayMs = s.DelayMs
	case *actions.SaveVariable:
		s := src.(*actions.SaveVariable)
		d.VariableName = s.VariableName
		d.Destination = s.Destination
		d.Append = s.Append
		d.AppendNewline = s.AppendNewline
	case *actions.Pause:
		s := src.(*actions.Pause)
		d.Message = s.Message
		d.PassThrough = s.PassThrough
	case *actions.ForEachRow:
		s := src.(*actions.ForEachRow)
		d.Name = s.Name
		d.StartRow = s.StartRow
		d.EndRow = s.EndRow
	case *actions.FocusWindow:
		s := src.(*actions.FocusWindow)
		d.WindowTitle = s.WindowTitle
		d.ProcessPath = s.ProcessPath
	}
	actions.RestoreUID(dst, uid)
}
