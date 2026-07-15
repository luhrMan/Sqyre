package macro

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type tooltipEditForm struct {
	toolbar          fyne.CanvasObject
	coordEditActions fyne.CanvasObject
	paramPills       fyne.CanvasObject
	targetItems      fyne.CanvasObject
	baseline         map[string]any

	stagedCoordRef actions.CoordinateRef
	coordApply       func(actions.CoordinateRef)

	applyAction func() error
}

func buildTooltipEditForm(node actions.ActionInterface, actionType string, owner *actionDisplayTooltipHover) *tooltipEditForm {
	form := &tooltipEditForm{
		applyAction: func() error { return nil },
	}
	var applyParts []func() error

	if is, ok := node.(*actions.ImageSearch); ok {
		targetBox, applyTargets := buildImageSearchTargetEdit(is, owner)
		form.targetItems = targetBox
		applyParts = append(applyParts, applyTargets)
	}

	form.paramPills, applyParts = buildParamEditPills(node, actionType, owner, applyParts)
	if len(applyParts) > 0 {
		form.applyAction = chainApply(applyParts...)
	}

	form.baseline, _ = snapshotActionMap(node)
	if binding, ok := actionCoordinateBinding(node); ok {
		form.stagedCoordRef = binding.ref
		form.coordApply = binding.set
	}
	form.coordEditActions = buildCoordEditActions(node, owner, form)
	form.toolbar = editToolbar(owner, form)
	return form
}

func chainApply(parts ...func() error) func() error {
	return func() error {
		for _, part := range parts {
			if part == nil {
				continue
			}
			if err := part(); err != nil {
				return err
			}
		}
		return nil
	}
}

func (form *tooltipEditForm) saveAction(owner *actionDisplayTooltipHover) error {
	if form.applyAction != nil {
		if err := form.applyAction(); err != nil {
			return err
		}
	}
	if form.coordApply != nil {
		form.coordApply(form.stagedCoordRef)
	}
	if err := validateTooltipAction(owner.node); err != nil {
		return err
	}
	if owner.onActionSaved != nil {
		owner.onActionSaved()
	}
	return nil
}

func editToolbar(owner *actionDisplayTooltipHover, form *tooltipEditForm) fyne.CanvasObject {
	actionType := owner.actionType
	actionSave := actiondisplay.NewPillIconButton(theme.DocumentSaveIcon(), func() {
		if owner.tooltipPanel != nil {
			owner.tooltipPanel.submitEdit()
		}
	})

	cancelBtn := actiondisplay.NewPillIconButton(theme.CancelIcon(), func() {
		owner.hideTooltip()
	})

	objects := []fyne.CanvasObject{}
	if pill := actionTooltipEditTypePill(actionType); pill != nil {
		objects = append(objects, pill)
	}
	objects = append(objects,
		layout.NewSpacer(),
		actiondisplay.PillChrome(actionSave, actionType),
		actiondisplay.PillChrome(cancelBtn, actionType),
	)
	return container.NewHBox(objects...)
}

func buildCoordEditActions(node actions.ActionInterface, owner *actionDisplayTooltipHover, form *tooltipEditForm) fyne.CanvasObject {
	if _, ok := actionCoordinateBinding(node); !ok || form.coordApply == nil {
		return nil
	}
	isPoint := actionUsesPointPicker(node)
	label := coordPickerButtonLabel(form.stagedCoordRef, isPoint)
	btn := widget.NewButton(label, nil)
	btn.Importance = widget.LowImportance
	btn.OnTapped = func() {
		var pick func(actions.CoordinateRef, func(actions.CoordinateRef), func())
		if isPoint {
			pick = activeWire.ShowPointPicker
		} else {
			pick = activeWire.ShowSearchAreaPicker
		}
		if pick == nil || activeWire.Window == nil {
			return
		}
		resumeBackdrop := owner.suspendBackdropDismissForPicker(nil)
		pick(form.stagedCoordRef, func(ref actions.CoordinateRef) {
			if ref == form.stagedCoordRef {
				return
			}
			form.stagedCoordRef = ref
			btn.SetText(coordPickerButtonLabel(ref, isPoint))
			owner.previewLoader = previewLoaderForRef(node, ref)
			owner.reloadPreview()
			owner.relayoutTooltip()
		}, resumeBackdrop)
	}
	return container.NewCenter(btn)
}

func coordPickerButtonLabel(ref actions.CoordinateRef, isPoint bool) string {
	if ref.IsEmpty() {
		if isPoint {
			return "Select point…"
		}
		return "Select search area…"
	}
	return ref.DisplayLabel()
}

func coordEntry(text string) *custom_widgets.BorderlessEntry {
	e := custom_widgets.NewBorderlessEntry(macroVariableDefs)
	e.SetText(text)
	return e
}

func varNameEntry(text string) *custom_widgets.BorderlessVarNameEntry {
	e := custom_widgets.NewBorderlessVarNameEntry(macroVariableDefs)
	e.SetText(text)
	return e
}

func parseRowBoundValue(text string) any {
	text = strings.TrimSpace(text)
	if text == "" {
		return nil
	}
	if strings.HasPrefix(text, "${") {
		return text
	}
	if i, err := strconv.Atoi(text); err == nil {
		return i
	}
	return text
}

func appendWaitTilFoundPills(row *pillRow, cfg *actions.WaitTilFoundConfig, intervalUIMin int, actionType string) func() {
	modeVal := cfg.EffectiveRepeatMode()
	modeSelect := actiondisplay.NewPillSelect("Repeat mode", actions.RepeatModes, modeVal, actions.RepeatModeLabel)
	row.add(actiondisplay.WrapPillSelect(modeSelect, actionType))

	secondsMin := 0
	secondsVal := cfg.WaitTilFoundSeconds
	if secondsVal <= 0 {
		secondsVal = 10
	}
	secondsInc := actiondisplay.NewPillIntStepper("Timeout (s)", secondsVal, 1, &secondsMin, nil, actionType)
	row.add(actiondisplay.WrapPillStepper(secondsInc, actionType))

	intervalMin := intervalUIMin
	intervalVal := cfg.WaitTilFoundIntervalMs
	if intervalVal < intervalUIMin {
		intervalVal = intervalUIMin
		if intervalVal == 0 {
			intervalVal = 100
		}
	}
	intervalInc := actiondisplay.NewPillIntStepper("Interval (ms)", intervalVal, 100, &intervalMin, nil, actionType)
	row.add(actiondisplay.WrapPillStepper(intervalInc, actionType))

	maxMin := 1
	maxVal := cfg.MaxIterations
	if maxVal <= 0 {
		maxVal = 100
	}
	maxInc := actiondisplay.NewPillIntStepper("Max iterations", maxVal, 1, &maxMin, nil, actionType)
	row.add(actiondisplay.WrapPillStepper(maxInc, actionType))

	setModeEnabled := func(mode string) {
		if mode == actions.RepeatOnce {
			secondsInc.Disable()
			intervalInc.Disable()
			maxInc.Disable()
			return
		}
		secondsInc.Enable()
		intervalInc.Enable()
		if mode == actions.RepeatWhileFound {
			maxInc.Enable()
		} else {
			maxInc.Disable()
		}
	}
	modeSelect.OnChanged = setModeEnabled
	setModeEnabled(modeSelect.Value)

	return func() {
		cfg.RepeatMode = modeSelect.Value
		cfg.WaitTilFoundSeconds = secondsInc.Value
		cfg.WaitTilFoundIntervalMs = intervalInc.Value
		cfg.MaxIterations = maxInc.Value
	}
}

func wirePillToggleSection(toggle *actiondisplay.PillToggle, setEnabled func(bool)) {
	toggle.OnChanged = setEnabled
	setEnabled(toggle.Value)
}

func buildParamEditPills(node actions.ActionInterface, actionType string, owner *actionDisplayTooltipHover, applyParts []func() error) (fyne.CanvasObject, []func() error) {
	var sections []fyne.CanvasObject
	added := false

	switch a := node.(type) {
	case *actions.Move:
		moveSections, apply := appendMoveTooltipEdit(a, actionType)
		sections = append(sections, moveSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Click:
		clickSections, apply := appendClickTooltipEdit(a, actionType, owner)
		sections = append(sections, clickSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Key:
		keySections, apply := appendKeyTooltipEdit(a, actionType)
		sections = append(sections, keySections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Wait:
		waitSections, apply := appendWaitTooltipEdit(a, actionType)
		sections = append(sections, waitSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Loop:
		loopSections, apply := appendLoopTooltipEdit(a, actionType)
		sections = append(sections, loopSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Conditional:
		condSections, apply := appendConditionalTooltipEdit(a, actionType, owner)
		sections = append(sections, condSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.SetVariable:
		setSections, apply := appendSetVariableTooltipEdit(a, actionType, owner)
		sections = append(sections, setSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.RunMacro:
		runSections, apply := appendRunMacroTooltipEdit(a, actionType, owner)
		sections = append(sections, runSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.ImageSearch:
		general := newPillRow()
		nameEntry := addNamePill(general, a.Name, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		match := newPillRow()
		tolMin, tolMax := 0.0, 1.0
		blurMin, blurMax := 1, 30
		tolInc := actiondisplay.NewPillFloatStepper("Tolerance", float64(a.Tolerance), 0.01, &tolMin, &tolMax, 2, actionType)
		blurInc := actiondisplay.NewPillIntStepper("Blur", a.Blur, 2, &blurMin, &blurMax, actionType)
		match.add(actiondisplay.WrapPillStepper(tolInc, actionType))
		match.add(actiondisplay.WrapPillStepper(blurInc, actionType))
		sections = append(sections, wrapTooltipSection(match.box))

		wait := newPillRow()
		applyWait := appendWaitTilFoundPills(wait, &a.WaitTilFoundConfig, 100, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		behavior := newPillRow()
		runOnNoFind := actiondisplay.NewPillToggle("Run on no find", a.RunBranchOnNoFind)
		behavior.add(actiondisplay.WrapPillToggle(runOnNoFind, actionType))
		sections = append(sections, wrapTooltipSection(behavior.box))

		outputs := newPillRow()
		outXEntry := varNameEntry(a.OutputXVariable)
		outYEntry := varNameEntry(a.OutputYVariable)
		outputs.add(actiondisplay.NewEditablePill("Output X", outXEntry, actionType))
		outputs.add(actiondisplay.NewEditablePill("Output Y", outYEntry, actionType))
		sections = append(sections, wrapTooltipSection(outputs.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Name = strings.TrimSpace(nameEntry.Text)
			a.Tolerance = float32(tolInc.Value)
			a.Blur = blurInc.Value
			applyWait()
			a.RunBranchOnNoFind = runOnNoFind.Value
			a.OutputXVariable = strings.TrimSpace(outXEntry.Text)
			a.OutputYVariable = strings.TrimSpace(outYEntry.Text)
			return nil
		})

	case *actions.FindPixel:
		general := newPillRow()
		nameEntry := addNamePill(general, a.Name, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		search := newPillRow()
		tolMin, tolMax := 0, 100
		tolInc := actiondisplay.NewPillIntStepper("Tolerance", a.ColorTolerance, 1, &tolMin, &tolMax, actionType)
		colorEntry := coordEntry(a.TargetColor)
		search.add(actiondisplay.NewEditablePill("Color", colorEntry, actionType))
		swatchPill, updateSwatch := editableColorSwatchPill(a.TargetColor, actionType)
		search.add(swatchPill)
		colorEntry.ChangedFn = func(text string) {
			updateSwatch(text)
			if owner != nil {
				owner.refreshTooltipLayout()
			}
		}
		if dropper := findPixelColorDropperButton(colorEntry, func() {
			updateSwatch(colorEntry.Text)
			if owner != nil {
				owner.refreshTooltipLayout()
			}
		}); dropper != nil {
			search.add(dropper)
		}
		search.add(actiondisplay.WrapPillStepper(tolInc, actionType))
		sections = append(sections, wrapTooltipSection(search.box))

		wait := newPillRow()
		applyWait := appendWaitTilFoundPills(wait, &a.WaitTilFoundConfig, 100, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		outputs := newPillRow()
		outXEntry := varNameEntry(a.OutputXVariable)
		outYEntry := varNameEntry(a.OutputYVariable)
		outputs.add(actiondisplay.NewEditablePill("Output X", outXEntry, actionType))
		outputs.add(actiondisplay.NewEditablePill("Output Y", outYEntry, actionType))
		sections = append(sections, wrapTooltipSection(outputs.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Name = strings.TrimSpace(nameEntry.Text)
			a.TargetColor = strings.ToLower(strings.TrimPrefix(strings.TrimSpace(colorEntry.Text), "#"))
			a.ColorTolerance = tolInc.Value
			applyWait()
			a.OutputXVariable = strings.TrimSpace(outXEntry.Text)
			a.OutputYVariable = strings.TrimSpace(outYEntry.Text)
			return nil
		})

	case *actions.Ocr:
		general := newPillRow()
		nameEntry := addNamePill(general, a.Name, actionType)
		targetEntry := coordEntry(a.Target)
		general.add(actiondisplay.NewEditablePill("Target", targetEntry, actionType))
		sections = append(sections, wrapTooltipSection(general.box))

		outputs := newPillRow()
		outVarEntry := varNameEntry(a.OutputVariable)
		outXEntry := varNameEntry(a.OutputXVariable)
		outYEntry := varNameEntry(a.OutputYVariable)
		outputs.add(actiondisplay.NewEditablePill("Output", outVarEntry, actionType))
		outputs.add(actiondisplay.NewEditablePill("Output X", outXEntry, actionType))
		outputs.add(actiondisplay.NewEditablePill("Output Y", outYEntry, actionType))
		sections = append(sections, wrapTooltipSection(outputs.box))

		preprocess := newPillRow()
		grayToggle := actiondisplay.NewPillToggle("Grayscale", a.Grayscale)
		blurMin, blurMax := 1, 30
		blurInc := actiondisplay.NewPillIntStepper("Blur", a.Blur, 2, &blurMin, &blurMax, actionType)
		thMin, thMax := 0, 255
		thInc := actiondisplay.NewPillIntStepper("Threshold", a.MinThreshold, 5, &thMin, &thMax, actionType)
		otsuToggle := actiondisplay.NewPillToggle("Auto threshold", a.ThresholdOtsu)
		invertToggle := actiondisplay.NewPillToggle("Invert threshold", a.ThresholdInvert)
		resizeMin, resizeMax := 1.0, 10.0
		resizeInc := actiondisplay.NewPillFloatStepper("Resize", a.Resize, 0.5, &resizeMin, &resizeMax, 1, actionType)
		preprocess.add(actiondisplay.WrapPillToggle(grayToggle, actionType))
		preprocess.add(actiondisplay.WrapPillStepper(blurInc, actionType))
		preprocess.add(actiondisplay.WrapPillStepper(thInc, actionType))
		preprocess.add(actiondisplay.WrapPillToggle(otsuToggle, actionType))
		preprocess.add(actiondisplay.WrapPillToggle(invertToggle, actionType))
		preprocess.add(actiondisplay.WrapPillStepper(resizeInc, actionType))
		setThresholdEnabled := func(auto bool) {
			if auto {
				thInc.Disable()
				return
			}
			thInc.Enable()
		}
		otsuToggle.OnChanged = setThresholdEnabled
		setThresholdEnabled(otsuToggle.Value)
		sections = append(sections, wrapTooltipSection(preprocess.box))

		wait := newPillRow()
		applyWait := appendWaitTilFoundPills(wait, &a.WaitTilFoundConfig, 0, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Name = strings.TrimSpace(nameEntry.Text)
			a.Target = strings.TrimSpace(targetEntry.Text)
			a.OutputVariable = strings.TrimSpace(outVarEntry.Text)
			a.OutputXVariable = strings.TrimSpace(outXEntry.Text)
			a.OutputYVariable = strings.TrimSpace(outYEntry.Text)
			a.Grayscale = grayToggle.Value
			a.Blur = blurInc.Value
			a.MinThreshold = thInc.Value
			a.ThresholdOtsu = otsuToggle.Value
			a.ThresholdInvert = invertToggle.Value
			a.Resize = resizeInc.Value
			applyWait()
			return nil
		})

	case *actions.Type:
		row := newPillRow()
		textEntry := coordEntry(a.Text)
		row.add(actiondisplay.NewEditablePill("Text", textEntry, actionType))
		delayMin, delayMax := 0, 60000
		delayInc := actiondisplay.NewPillIntStepper("Delay (ms)", a.DelayMs, 1, &delayMin, &delayMax, actionType)
		row.add(actiondisplay.WrapPillStepper(delayInc, actionType))
		sections = append(sections, wrapTooltipSection(row.box))
		added = true
		applyParts = append(applyParts, func() error {
			a.Text = textEntry.Text
			a.DelayMs = delayInc.Value
			return nil
		})

	case *actions.SaveVariable:
		output := newPillRow()
		varEntry := varNameEntry(a.VariableName)
		destEntry := coordEntry(a.Destination)
		output.add(actiondisplay.NewEditablePill("Variable", varEntry, actionType))
		output.add(actiondisplay.NewEditablePill("Destination", destEntry, actionType))
		sections = append(sections, wrapTooltipSection(output.box))

		fileOpts := newPillRow()
		appendToggle := actiondisplay.NewPillToggle("Append", a.Append)
		newlineToggle := actiondisplay.NewPillToggle("Append newline", a.AppendNewline)
		fileOpts.add(actiondisplay.WrapPillToggle(appendToggle, actionType))
		fileOpts.add(actiondisplay.WrapPillToggle(newlineToggle, actionType))
		wirePillToggleSection(appendToggle, func(enabled bool) {
			if enabled {
				newlineToggle.Enable()
				return
			}
			newlineToggle.Disable()
		})
		sections = append(sections, wrapTooltipSection(fileOpts.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.VariableName = strings.TrimSpace(varEntry.Text)
			a.Destination = destEntry.Text
			a.Append = appendToggle.Value
			a.AppendNewline = newlineToggle.Value
			return nil
		})

	case *actions.Pause:
		message := newPillRow()
		messageEntry := coordEntry(a.Message)
		message.add(actiondisplay.NewEditablePill("Message", messageEntry, actionType))
		sections = append(sections, wrapTooltipSection(message.box))

		key := newPillRow()
		tempKeys := append([]string(nil), a.ContinueKey...)
		keyLabel := widget.NewLabel(macrohotkey.FormatContinueKey(tempKeys))
		if keyLabel.Text == "" {
			keyLabel.SetText("(not set)")
		}
		if activeWire.ShowHotkeyRecordDialog != nil && activeWire.Window != nil {
			recordBtn := actiondisplay.NewPillIconButton(theme.NewErrorThemedResource(theme.MediaRecordIcon()), func() {
				activeWire.ShowHotkeyRecordDialog(activeWire.Window, time.Second, func(recorded []string) {
					tempKeys = append([]string(nil), recorded...)
					keyLabel.SetText(macrohotkey.FormatContinueKey(tempKeys))
					if keyLabel.Text == "" {
						keyLabel.SetText("(not set)")
					}
					if owner != nil {
						owner.refreshTooltipLayout()
					}
				})
			})
			key.add(actiondisplay.PillChrome(container.NewVBox(keyLabel, recordBtn), actionType))
		} else {
			key.add(actiondisplay.PillChrome(keyLabel, actionType))
		}
		passToggle := actiondisplay.NewPillToggle("Pass through", a.PassThrough)
		key.add(actiondisplay.WrapPillToggle(passToggle, actionType))
		sections = append(sections, wrapTooltipSection(key.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Message = messageEntry.Text
			a.ContinueKey = append([]string(nil), tempKeys...)
			a.PassThrough = passToggle.Value
			return nil
		})

	case *actions.ForEachRow:
		return appendForEachRowTooltipEdit(a, actionType, owner, applyParts)

	case *actions.FocusWindow:
		row := newPillRow()
		titleEntry := coordEntry(a.WindowTitle)
		pathEntry := coordEntry(a.ProcessPath)
		row.add(actiondisplay.NewEditablePill("Title", titleEntry, actionType))
		row.add(actiondisplay.NewEditablePill("App", pathEntry, actionType))
		row.add(windowPickerButton(actionType, func(title, path string) {
			titleEntry.SetText(title)
			pathEntry.SetText(path)
			if owner != nil {
				owner.refreshTooltipLayout()
			}
		}))
		sections = append(sections, wrapTooltipSection(row.box))
		added = true
		applyParts = append(applyParts, func() error {
			a.WindowTitle = strings.TrimSpace(titleEntry.Text)
			a.ProcessPath = strings.TrimSpace(pathEntry.Text)
			return nil
		})
	}

	if !added {
		return nil, applyParts
	}
	return joinTooltipSections(sections...), applyParts
}

func addNamePill(row *pillRow, name, actionType string) *custom_widgets.BorderlessEntry {
	entry := coordEntry(name)
	row.add(actiondisplay.NewEditablePill("Name", entry, actionType))
	return entry
}

func viewParamPillsContentKey(node actions.ActionInterface) string {
	if node == nil {
		return ""
	}
	m, err := snapshotActionMap(node)
	if err != nil || m == nil {
		return node.GetType()
	}
	b, err := json.Marshal(m)
	if err != nil {
		return node.GetType()
	}
	return string(b)
}

func viewParamPills(node actions.ActionInterface, actionType string) fyne.CanvasObject {
	var sections []fyne.CanvasObject
	added := false

	switch a := node.(type) {
	case *actions.Move:
		sections = append(sections, appendMoveTooltipView(a, actionType)...)
		added = true

	case *actions.Click:
		sections = append(sections, appendClickTooltipView(a, actionType)...)
		added = true

	case *actions.Key:
		sections = append(sections, appendKeyTooltipView(a, actionType)...)
		added = true

	case *actions.Wait:
		sections = append(sections, appendWaitTooltipView(a, actionType)...)
		added = true

	case *actions.Loop:
		sections = append(sections, appendLoopTooltipView(a, actionType)...)
		added = true

	case *actions.Conditional:
		sections = append(sections, appendConditionalTooltipView(a, actionType)...)
		added = true

	case *actions.SetVariable:
		sections = append(sections, appendSetVariableTooltipView(a, actionType)...)
		added = true

	case *actions.RunMacro:
		sections = append(sections, appendRunMacroTooltipView(a, actionType)...)
		added = true

	case *actions.Break:
		sections = append(sections, appendFlowControlTooltipView(actionType, "Exits the innermost enclosing loop.")...)
		added = true

	case *actions.Continue:
		sections = append(sections, appendFlowControlTooltipView(actionType, "Skips to the next loop iteration.")...)
		added = true

	case *actions.ImageSearch:
		general := newPillRow()
		addDisplayPill(general, "Name", a.Name, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		match := newPillRow()
		match.add(actiondisplay.NewDisplayPill("Tolerance: "+actions.FormatParamValue(a.Tolerance), actionType))
		match.add(actiondisplay.NewDisplayPill("Blur: "+actions.FormatParamValue(a.Blur), actionType))
		sections = append(sections, wrapTooltipSection(match.box))

		wait := newPillRow()
		appendWaitTilFoundViewPills(wait, &a.WaitTilFoundConfig, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		behavior := newPillRow()
		behavior.add(actiondisplay.NewDisplayTogglePill("Run on no find", a.RunBranchOnNoFind, actionType))
		sections = append(sections, wrapTooltipSection(behavior.box))

		outputs := newPillRow()
		addDisplayVariablePill(outputs, "Output X", a.OutputXVariable, actionType)
		addDisplayVariablePill(outputs, "Output Y", a.OutputYVariable, actionType)
		sections = append(sections, wrapTooltipSection(outputs.box))
		added = true

	case *actions.FindPixel:
		general := newPillRow()
		addDisplayPill(general, "Name", a.Name, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		search := newPillRow()
		addDisplayPill(search, "Color", a.TargetColor, actionType)
		if swatch := colorSwatchPill(a.TargetColor, actionType); swatch != nil {
			search.add(swatch)
		}
		search.add(actiondisplay.NewDisplayPill("Tolerance: "+actions.FormatParamValue(a.ColorTolerance), actionType))
		sections = append(sections, wrapTooltipSection(search.box))

		wait := newPillRow()
		appendWaitTilFoundViewPills(wait, &a.WaitTilFoundConfig, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		outputs := newPillRow()
		addDisplayVariablePill(outputs, "Output X", a.OutputXVariable, actionType)
		addDisplayVariablePill(outputs, "Output Y", a.OutputYVariable, actionType)
		sections = append(sections, wrapTooltipSection(outputs.box))
		added = true

	case *actions.Ocr:
		general := newPillRow()
		addDisplayPill(general, "Name", a.Name, actionType)
		addDisplayPill(general, "Target", a.Target, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		outputs := newPillRow()
		addDisplayVariablePill(outputs, "Output", a.OutputVariable, actionType)
		addDisplayVariablePill(outputs, "Output X", a.OutputXVariable, actionType)
		addDisplayVariablePill(outputs, "Output Y", a.OutputYVariable, actionType)
		sections = append(sections, wrapTooltipSection(outputs.box))

		wait := newPillRow()
		appendWaitTilFoundViewPills(wait, &a.WaitTilFoundConfig, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))
		added = true

	case *actions.Type:
		row := newPillRow()
		addDisplayPill(row, "Text", a.Text, actionType)
		row.add(actiondisplay.NewDisplayPill("Delay (ms): "+actions.FormatParamValue(a.DelayMs), actionType))
		sections = append(sections, wrapTooltipSection(row.box))
		added = true

	case *actions.SaveVariable:
		output := newPillRow()
		addDisplayVariablePill(output, "Variable", a.VariableName, actionType)
		addDisplayPill(output, "Destination", a.Destination, actionType)
		sections = append(sections, wrapTooltipSection(output.box))

		fileOpts := newPillRow()
		fileOpts.add(actiondisplay.NewDisplayTogglePill("Append", a.Append, actionType))
		fileOpts.add(actiondisplay.NewDisplayTogglePill("Append newline", a.AppendNewline, actionType))
		sections = append(sections, wrapTooltipSection(fileOpts.box))
		added = true

	case *actions.Pause:
		message := newPillRow()
		addDisplayPill(message, "Message", a.Message, actionType)
		sections = append(sections, wrapTooltipSection(message.box))

		key := newPillRow()
		key.add(actiondisplay.NewDisplayTogglePill("Pass through", a.PassThrough, actionType))
		sections = append(sections, wrapTooltipSection(key.box))
		added = true

	case *actions.ForEachRow:
		general := newPillRow()
		addDisplayPill(general, "Name", a.Name, actionType)
		addDisplayPill(general, "Start row", formatAnyValue(a.StartRow), actionType)
		addDisplayPill(general, "End row", formatAnyValue(a.EndRow), actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		for i, s := range a.Sources {
			srcRow := newPillRow()
			addDisplayPill(srcRow, fmt.Sprintf("Source %d", i+1), s.Source, actionType)
			addDisplayVariablePill(srcRow, "Output", s.OutputVar, actionType)
			srcRow.add(actiondisplay.NewDisplayTogglePill("Is file", s.IsFile, actionType))
			srcRow.add(actiondisplay.NewDisplayTogglePill("Skip blank", s.SkipBlankLines, actionType))
			sections = append(sections, wrapTooltipSection(srcRow.box))
		}
		added = true

	case *actions.FocusWindow:
		row := newPillRow()
		addDisplayPill(row, "Title", a.WindowTitle, actionType)
		addDisplayPill(row, "App", a.ProcessPath, actionType)
		sections = append(sections, wrapTooltipSection(row.box))
		added = true
	}

	if !added {
		return nil
	}
	return joinTooltipSections(sections...)
}

func appendWaitTilFoundViewPills(row *pillRow, cfg *actions.WaitTilFoundConfig, actionType string) {
	addInlineDisplayPill(row, "Repeat mode", actions.RepeatModeLabel(cfg.EffectiveRepeatMode()), actionType)
	addInlineDisplayPill(row, "Timeout (s)", actions.FormatParamValue(cfg.WaitTilFoundSeconds), actionType)
	addInlineDisplayPill(row, "Interval (ms)", actions.FormatParamValue(cfg.WaitTilFoundIntervalMs), actionType)
	if cfg.IsRepeatWhileFound() {
		addInlineDisplayPill(row, "Max iterations", actions.FormatParamValue(cfg.EffectiveMaxIterations()), actionType)
	}
}
