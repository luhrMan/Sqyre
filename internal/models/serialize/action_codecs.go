package serialize

import (
	"Sqyre/internal/models/actions"
	"fmt"
)

func init() {
	registerActionCodec("loop", decodeLoop, encodeLoop)
	registerActionCodec("wait", decodeWait, encodeWait)
	registerActionCodec("pause", decodePause, encodePause)
	registerActionCodec("findpixel", decodeFindPixel, encodeFindPixel)
	registerActionCodec("click", decodeClick, encodeClick)
	registerActionCodec("move", decodeMove, encodeMove)
	registerActionCodec("key", decodeKey, encodeKey)
	registerActionCodec("type", decodeType, encodeType)
	registerActionCodec("imagesearch", decodeImageSearch, encodeImageSearch)
	registerActionCodec("ocr", decodeOcr, encodeOcr)
	registerActionCodec("semanticsearch", decodeSemanticSearch, encodeSemanticSearch)
	registerActionCodec("setvariable", decodeSetVariable, encodeSetVariable)
	registerActionCodec("calculate", decodeCalculate, encodeCalculate)
	registerActionCodec("conditional", decodeConditional, encodeConditional)
	registerActionCodec("foreachrow", decodeForEachRow, encodeForEachRow)
	registerActionCodec("savevariable", decodeSaveVariable, encodeSaveVariable)
	registerActionCodec("focuswindow", decodeFocusWindow, encodeFocusWindow)
	registerActionCodec("runmacro", decodeRunMacro, encodeRunMacro)
	registerActionCodec("break", decodeBreak, encodeBreak)
	registerActionCodec("continue", decodeContinue, encodeContinue)
}

func decodeLoop(rawMap map[string]any) (actions.ActionInterface, error) {
	countVal := rawMap["count"]
	if countVal == nil {
		countVal = 1
	}
	name, err := expectString(rawMap, "name")
	if err != nil {
		return nil, fmt.Errorf("action type loop: %w", err)
	}
	return actions.NewLoop(countVal, name, []actions.ActionInterface{}), nil
}

func encodeLoop(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Loop)
	m := map[string]any{"type": "loop", "count": a.Count, "name": a.Name}
	subs, err := subActionsToMaps(a.GetSubActions())
	if err != nil {
		return nil, err
	}
	m["subactions"] = subs
	return m, nil
}

func decodeWait(rawMap map[string]any) (actions.ActionInterface, error) {
	timeVal := rawMap["time"]
	if timeVal == nil {
		timeVal = 0
	}
	return actions.NewWait(timeVal), nil
}

func encodeWait(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Wait)
	return map[string]any{"type": "wait", "time": a.Time}, nil
}

func decodePause(rawMap map[string]any) (actions.ActionInterface, error) {
	passThrough := false
	if v, ok := rawMap["passthrough"].(bool); ok {
		passThrough = v
	}
	return actions.NewPause(
		stringFromMap(rawMap, "message"),
		stringSliceFromAny(rawMap["continuekey"]),
		passThrough,
	), nil
}

func encodePause(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Pause)
	m := map[string]any{"type": "pause"}
	if a.Message != "" {
		m["message"] = a.Message
	}
	if len(a.ContinueKey) > 0 {
		m["continuekey"] = a.ContinueKey
	}
	if a.PassThrough {
		m["passthrough"] = a.PassThrough
	}
	return m, nil
}

func decodeFindPixel(rawMap map[string]any) (actions.ActionInterface, error) {
	name := stringFromMap(rawMap, "name")
	searchArea := parseCoordinateRef(rawMap["searcharea"])
	targetColor := stringFromMap(rawMap, "targetcolor")
	if targetColor == "" {
		targetColor = "ffffff"
	}
	colorTolerance := intFromMap(rawMap["colortolerance"])
	if colorTolerance < 0 || colorTolerance > 100 {
		colorTolerance = 0
	}
	fp := actions.NewFindPixel(name, searchArea, targetColor, colorTolerance)
	decodeCoordinateOutputs(rawMap, &fp.CoordinateOutputs)
	decodeWaitTilFound(rawMap, &fp.WaitTilFoundConfig)
	return fp, nil
}

func encodeFindPixel(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.FindPixel)
	m := map[string]any{
		"type":           "findpixel",
		"name":           a.Name,
		"searcharea":     coordinateRefToMap(a.SearchArea),
		"targetcolor":    a.TargetColor,
		"colortolerance": a.ColorTolerance,
	}
	writeCoordinateOutputs(m, a.CoordinateOutputs)
	writeWaitTilFound(m, a.WaitTilFoundConfig)
	return m, nil
}

func decodeClick(rawMap map[string]any) (actions.ActionInterface, error) {
	button, err := expectString(rawMap, "button")
	if err != nil {
		return nil, fmt.Errorf("action type click: %w", err)
	}
	switch button {
	case actions.ClickButtonLeft, actions.ClickButtonRight, actions.ClickButtonCenter, actions.ClickButtonScroll:
	default:
		return nil, fmt.Errorf("action type click: field %q: unknown button %q", "button", button)
	}
	state := false
	if v, ok := rawMap["state"].(bool); ok {
		state = v
	}
	return actions.NewClick(button, state), nil
}

func encodeClick(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Click)
	return map[string]any{"type": "click", "button": a.Button, "state": a.State}, nil
}

func decodeMove(rawMap map[string]any) (actions.ActionInterface, error) {
	pointRef := parseCoordinateRef(rawMap["point"])
	smooth := false
	if v, ok := rawMap["smooth"].(bool); ok {
		smooth = v
	}
	move := actions.NewMove(pointRef, smooth)
	if v := rawMap["smoothlow"]; v != nil {
		move.SmoothLow = floatFromMap(v)
	}
	if v := rawMap["smoothhigh"]; v != nil {
		move.SmoothHigh = floatFromMap(v)
	}
	if v := rawMap["smoothdelayms"]; v != nil {
		move.SmoothDelayMs = intFromMap(v)
	}
	return move, nil
}

func encodeMove(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Move)
	m := map[string]any{
		"type":   "move",
		"point":  coordinateRefToMap(a.Point),
		"smooth": a.Smooth,
	}
	if a.Smooth {
		m["smoothlow"] = a.EffectiveSmoothLow()
		m["smoothhigh"] = a.EffectiveSmoothHigh()
		m["smoothdelayms"] = a.EffectiveSmoothDelayMs()
	}
	return m, nil
}

func decodeKey(rawMap map[string]any) (actions.ActionInterface, error) {
	k, err := expectString(rawMap, "key")
	if err != nil {
		return nil, fmt.Errorf("action type key: %w", err)
	}
	st, err := expectBool(rawMap, "state")
	if err != nil {
		return nil, fmt.Errorf("action type key: %w", err)
	}
	return actions.NewKey(k, st), nil
}

func encodeKey(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Key)
	return map[string]any{"type": "key", "key": a.Key, "state": a.State}, nil
}

func decodeType(rawMap map[string]any) (actions.ActionInterface, error) {
	text := stringFromMap(rawMap, "text")
	delayMs := 0
	if v := rawMap["delayms"]; v != nil {
		delayMs = intFromMap(v)
	}
	return actions.NewType(text, delayMs), nil
}

func encodeType(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Type)
	return map[string]any{"type": "type", "text": a.Text, "delayms": a.DelayMs}, nil
}

func decodeImageSearch(rawMap map[string]any) (actions.ActionInterface, error) {
	targets := targetsFromMap(rawMap["targets"])
	blur := 5
	if v, ok := rawMap["blur"]; ok {
		blur = intFromMap(v)
	}
	name, err := expectString(rawMap, "name")
	if err != nil {
		return nil, fmt.Errorf("action type imagesearch: %w", err)
	}
	sa := parseCoordinateRef(rawMap["searcharea"])
	row, err := expectInt(rawMap, "rowsplit")
	if err != nil {
		return nil, fmt.Errorf("action type imagesearch: %w", err)
	}
	col, err := expectInt(rawMap, "colsplit")
	if err != nil {
		return nil, fmt.Errorf("action type imagesearch: %w", err)
	}
	tol, err := expectFloat64(rawMap, "tolerance")
	if err != nil {
		return nil, fmt.Errorf("action type imagesearch: %w", err)
	}
	is := actions.NewImageSearch(name, []actions.ActionInterface{}, targets, sa, row, col, float32(tol), blur)
	decodeCoordinateOutputs(rawMap, &is.CoordinateOutputs)
	decodeWaitTilFound(rawMap, &is.WaitTilFoundConfig)
	if v, ok := rawMap["runbranchonnofind"].(bool); ok {
		is.RunBranchOnNoFind = v
	}
	return is, nil
}

func encodeImageSearch(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.ImageSearch)
	m := map[string]any{
		"type":       "imagesearch",
		"name":       a.Name,
		"targets":    a.Targets,
		"searcharea": coordinateRefToMap(a.SearchArea),
		"rowsplit":   a.RowSplit,
		"colsplit":   a.ColSplit,
		"tolerance":  float64(a.Tolerance),
		"blur":       a.Blur,
	}
	writeCoordinateOutputs(m, a.CoordinateOutputs)
	writeWaitTilFound(m, a.WaitTilFoundConfig)
	if a.RunBranchOnNoFind {
		m["runbranchonnofind"] = a.RunBranchOnNoFind
	}
	subs, err := subActionsToMaps(a.GetSubActions())
	if err != nil {
		return nil, err
	}
	m["subactions"] = subs
	return m, nil
}

func decodeOcr(rawMap map[string]any) (actions.ActionInterface, error) {
	oname, err := expectString(rawMap, "name")
	if err != nil {
		return nil, fmt.Errorf("action type ocr: %w", err)
	}
	target, err := expectString(rawMap, "target")
	if err != nil {
		return nil, fmt.Errorf("action type ocr: %w", err)
	}
	sa := parseCoordinateRef(rawMap["searcharea"])
	oc := actions.NewOcr(oname, target, sa)
	if v, ok := rawMap["outputvariable"].(string); ok {
		oc.OutputVariable = v
	}
	decodeCoordinateOutputs(rawMap, &oc.CoordinateOutputs)
	decodeWaitTilFound(rawMap, &oc.WaitTilFoundConfig)
	if v, ok := rawMap["grayscale"].(bool); ok {
		oc.Grayscale = v
	}
	if v := rawMap["blur"]; v != nil {
		oc.Blur = intFromMap(v)
	}
	if oc.Blur < 1 {
		oc.Blur = 1
	}
	if v := rawMap["minthreshold"]; v != nil {
		oc.MinThreshold = intFromMap(v)
	}
	if v := rawMap["resize"]; v != nil {
		oc.Resize = floatFromMap(v)
	}
	if v, ok := rawMap["thresholdotsu"].(bool); ok {
		oc.ThresholdOtsu = v
	}
	if v, ok := rawMap["thresholdinvert"].(bool); ok {
		oc.ThresholdInvert = v
	}
	return oc, nil
}

func encodeOcr(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Ocr)
	m := map[string]any{
		"type":       "ocr",
		"name":       a.Name,
		"target":     a.Target,
		"searcharea": coordinateRefToMap(a.SearchArea),
	}
	if a.OutputVariable != "" {
		m["outputvariable"] = a.OutputVariable
	}
	writeCoordinateOutputs(m, a.CoordinateOutputs)
	writeWaitTilFound(m, a.WaitTilFoundConfig)
	if !a.Grayscale {
		m["grayscale"] = a.Grayscale
	}
	if a.Blur != 1 {
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
	return m, nil
}

func decodeSemanticSearch(rawMap map[string]any) (actions.ActionInterface, error) {
	name, err := expectString(rawMap, "name")
	if err != nil {
		return nil, fmt.Errorf("action type semanticsearch: %w", err)
	}
	prompt, err := expectString(rawMap, "prompt")
	if err != nil {
		return nil, fmt.Errorf("action type semanticsearch: %w", err)
	}
	sa := parseCoordinateRef(rawMap["searcharea"])
	ss := actions.NewSemanticSearch(name, []actions.ActionInterface{}, prompt, sa)
	decodeCoordinateOutputs(rawMap, &ss.CoordinateOutputs)
	decodeWaitTilFound(rawMap, &ss.WaitTilFoundConfig)
	if v, ok := rawMap["runbranchonnofind"].(bool); ok {
		ss.RunBranchOnNoFind = v
	}
	if v, ok := rawMap["outputlabelvariable"].(string); ok {
		ss.OutputLabelVariable = v
	}
	if v := rawMap["confidencethreshold"]; v != nil {
		ss.ConfidenceThreshold = float32(floatFromMap(v))
	}
	if v := rawMap["iouthreshold"]; v != nil {
		ss.IoUThreshold = float32(floatFromMap(v))
	}
	if v := rawMap["maxmatches"]; v != nil {
		ss.MaxMatches = intFromMap(v)
	}
	return ss, nil
}

func encodeSemanticSearch(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.SemanticSearch)
	m := map[string]any{
		"type":       "semanticsearch",
		"name":       a.Name,
		"prompt":     a.Prompt,
		"searcharea": coordinateRefToMap(a.SearchArea),
	}
	writeCoordinateOutputs(m, a.CoordinateOutputs)
	writeWaitTilFound(m, a.WaitTilFoundConfig)
	if a.ConfidenceThreshold != 0 && a.ConfidenceThreshold != 0.25 {
		m["confidencethreshold"] = a.ConfidenceThreshold
	}
	if a.IoUThreshold != 0 && a.IoUThreshold != 0.45 {
		m["iouthreshold"] = a.IoUThreshold
	}
	if a.MaxMatches > 0 {
		m["maxmatches"] = a.MaxMatches
	}
	if a.OutputLabelVariable != "" {
		m["outputlabelvariable"] = a.OutputLabelVariable
	}
	if a.RunBranchOnNoFind {
		m["runbranchonnofind"] = true
	}
	subs, err := subActionsToMaps(a.GetSubActions())
	if err != nil {
		return nil, err
	}
	m["subactions"] = subs
	return m, nil
}

func decodeSetVariable(rawMap map[string]any) (actions.ActionInterface, error) {
	vn, err := expectString(rawMap, "variablename")
	if err != nil {
		return nil, fmt.Errorf("action type setvariable: %w", err)
	}
	return actions.NewSetVariable(vn, rawMap["value"]), nil
}

func encodeSetVariable(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.SetVariable)
	return map[string]any{
		"type":         "setvariable",
		"variablename": a.VariableName,
		"value":        a.Value,
	}, nil
}

func decodeCalculate(rawMap map[string]any) (actions.ActionInterface, error) {
	expr, err := expectString(rawMap, "expression")
	if err != nil {
		return nil, fmt.Errorf("action type calculate: %w", err)
	}
	outv, err := expectString(rawMap, "outputvar")
	if err != nil {
		return nil, fmt.Errorf("action type calculate: %w", err)
	}
	return actions.NewCalculate(expr, outv), nil
}

func encodeCalculate(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Calculate)
	return map[string]any{
		"type":       "calculate",
		"expression": a.Expression,
		"outputvar":  a.OutputVar,
	}, nil
}

func decodeConditional(rawMap map[string]any) (actions.ActionInterface, error) {
	name := stringFromMap(rawMap, "name")
	clauses := clausesFromMap(rawMap)
	match := stringFromMap(rawMap, "match")
	return actions.NewConditional(clauses, match, name, []actions.ActionInterface{}), nil
}

func encodeConditional(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.Conditional)
	m := map[string]any{
		"type":  "conditional",
		"name":  a.Name,
		"match": a.EffectiveMatch(),
	}
	clauses := make([]map[string]any, len(a.Clauses))
	for i, c := range a.Clauses {
		clauses[i] = map[string]any{
			"left":     c.Left,
			"operator": c.Operator,
			"right":    c.Right,
		}
	}
	m["clauses"] = clauses
	subs, err := subActionsToMaps(a.GetSubActions())
	if err != nil {
		return nil, err
	}
	m["subactions"] = subs
	return m, nil
}

func decodeForEachRow(rawMap map[string]any) (actions.ActionInterface, error) {
	name := stringFromMap(rawMap, "name")
	sources := forEachRowSourcesFromMap(rawMap)
	fer := actions.NewForEachRow(name, sources, []actions.ActionInterface{})
	fer.StartRow = rowBoundFromMap(rawMap, "startrow")
	fer.EndRow = rowBoundFromMap(rawMap, "endrow")
	return fer, nil
}

func encodeForEachRow(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.ForEachRow)
	m := map[string]any{
		"type":    "foreachrow",
		"name":    a.Name,
		"sources": listColumnsToMaps(a.Sources),
	}
	if actions.RowBoundIsSet(a.StartRow) {
		m["startrow"] = a.StartRow
	}
	if actions.RowBoundIsSet(a.EndRow) {
		m["endrow"] = a.EndRow
	}
	subs, err := subActionsToMaps(a.GetSubActions())
	if err != nil {
		return nil, err
	}
	m["subactions"] = subs
	return m, nil
}

func decodeSaveVariable(rawMap map[string]any) (actions.ActionInterface, error) {
	append := false
	if appendVal, ok := rawMap["append"]; ok && appendVal != nil {
		b, ok := appendVal.(bool)
		if !ok {
			return nil, fmt.Errorf("action type savevariable: field \"append\": expected bool, got %T", appendVal)
		}
		append = b
	}
	appendNewline := false
	if nlVal, ok := rawMap["appendnewline"]; ok && nlVal != nil {
		b, ok := nlVal.(bool)
		if !ok {
			return nil, fmt.Errorf("action type savevariable: field \"appendnewline\": expected bool, got %T", nlVal)
		}
		appendNewline = b
	}
	vn, err := expectString(rawMap, "variablename")
	if err != nil {
		return nil, fmt.Errorf("action type savevariable: %w", err)
	}
	dest, err := expectString(rawMap, "destination")
	if err != nil {
		return nil, fmt.Errorf("action type savevariable: %w", err)
	}
	return actions.NewSaveVariable(vn, dest, append, appendNewline), nil
}

func encodeSaveVariable(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.SaveVariable)
	return map[string]any{
		"type":          "savevariable",
		"variablename":  a.VariableName,
		"destination":   a.Destination,
		"append":        a.Append,
		"appendnewline": a.AppendNewline,
	}, nil
}

func decodeFocusWindow(rawMap map[string]any) (actions.ActionInterface, error) {
	return actions.NewFocusWindow(
		stringFromMap(rawMap, "processpath"),
		stringFromMap(rawMap, "windowtitle"),
	), nil
}

func encodeFocusWindow(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.FocusWindow)
	return map[string]any{
		"type":        "focuswindow",
		"processpath": a.ProcessPath,
		"windowtitle": a.WindowTitle,
	}, nil
}

func decodeRunMacro(rawMap map[string]any) (actions.ActionInterface, error) {
	return actions.NewRunMacro(stringFromMap(rawMap, "macroname")), nil
}

func encodeRunMacro(action actions.ActionInterface) (map[string]any, error) {
	a := action.(*actions.RunMacro)
	return map[string]any{"type": "runmacro", "macroname": a.MacroName}, nil
}

func decodeBreak(map[string]any) (actions.ActionInterface, error) {
	return actions.NewBreak(), nil
}

func encodeBreak(actions.ActionInterface) (map[string]any, error) {
	return map[string]any{"type": "break"}, nil
}

func decodeContinue(map[string]any) (actions.ActionInterface, error) {
	return actions.NewContinue(), nil
}

func encodeContinue(actions.ActionInterface) (map[string]any, error) {
	return map[string]any{"type": "continue"}, nil
}
