package macro

import (
	"strconv"
	"strings"

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

	applyAction func() error
}

func buildTooltipEditForm(node actions.ActionInterface, actionType string, owner *actionDisplayTooltipHover) *tooltipEditForm {
	form := &tooltipEditForm{
		applyAction: func() error { return nil },
	}
	var applyParts []func() error

	if is, ok := node.(*actions.ImageSearch); ok && len(is.Targets) > 0 {
		targetBox, applyTargets := buildImageSearchTargetEdit(is, owner)
		form.targetItems = targetBox
		applyParts = append(applyParts, applyTargets)
	}

	form.paramPills, applyParts = buildParamEditPills(node, actionType, applyParts)
	if len(applyParts) > 0 {
		form.applyAction = chainApply(applyParts...)
	}

	form.baseline, _ = snapshotActionMap(node)
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

	return container.NewHBox(
		layout.NewSpacer(),
		actiondisplay.PillChrome(actionSave, actionType),
		actiondisplay.PillChrome(cancelBtn, actionType),
	)
}

func buildCoordEditActions(node actions.ActionInterface, owner *actionDisplayTooltipHover, form *tooltipEditForm) fyne.CanvasObject {
	binding, ok := actionCoordinateBinding(node)
	if !ok || binding.ref.IsEmpty() || activeWire.NavigateToCoordinateEntity == nil {
		return nil
	}
	isPoint := actionUsesPointPicker(node)
	ref := binding.ref
	navigate := func() {
		activeWire.NavigateToCoordinateEntity(ref, isPoint)
		owner.hideTooltip()
	}
	navBtn := widget.NewButton("Edit in Data Editor", func() {
		if form.hasPendingChanges(node) {
			if activeWire.ShowConfirmWithEscape != nil && activeWire.Window != nil {
				activeWire.ShowConfirmWithEscape(
					"Unsaved Changes",
					"Save changes before opening the Data Editor?",
					func(save bool) {
						if save {
							if err := form.saveAction(owner); err != nil {
								if activeWire.ShowErrorWithEscape != nil {
									activeWire.ShowErrorWithEscape(err, activeWire.Window)
								}
								return
							}
						}
						navigate()
					},
					activeWire.Window,
				)
				return
			}
		}
		navigate()
	})
	navBtn.Importance = widget.LowImportance
	return container.NewCenter(navBtn)
}

func coordEntry(text string) *custom_widgets.BorderlessEntry {
	e := custom_widgets.NewBorderlessEntry(macroVariableDefs)
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
	waitToggle := actiondisplay.NewPillToggle("Wait until found", cfg.WaitTilFound)
	row.add(actiondisplay.WrapPillToggle(waitToggle, actionType))

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

	setWaitEnabled := func(enabled bool) {
		if enabled {
			secondsInc.Enable()
			intervalInc.Enable()
			return
		}
		secondsInc.Disable()
		intervalInc.Disable()
	}
	waitToggle.OnChanged = setWaitEnabled
	setWaitEnabled(waitToggle.Value)

	return func() {
		cfg.WaitTilFound = waitToggle.Value
		cfg.WaitTilFoundSeconds = secondsInc.Value
		cfg.WaitTilFoundIntervalMs = intervalInc.Value
	}
}

func wirePillToggleSection(toggle *actiondisplay.PillToggle, setEnabled func(bool)) {
	toggle.OnChanged = setEnabled
	setEnabled(toggle.Value)
}

func buildParamEditPills(node actions.ActionInterface, actionType string, applyParts []func() error) (fyne.CanvasObject, []func() error) {
	var sections []fyne.CanvasObject
	added := false

	switch a := node.(type) {
	case *actions.Move:
		moveSections, apply := appendMoveTooltipEdit(a, actionType)
		sections = append(sections, moveSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Click:
		clickSections, apply := appendClickTooltipEdit(a, actionType)
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
		condSections, apply := appendConditionalTooltipEdit(a, actionType)
		sections = append(sections, condSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.SetVariable:
		setSections, apply := appendSetVariableTooltipEdit(a, actionType)
		sections = append(sections, setSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.Calculate:
		calcSections, apply := appendCalculateTooltipEdit(a, actionType)
		sections = append(sections, calcSections...)
		applyParts = append(applyParts, apply)
		added = true

	case *actions.RunMacro:
		runSections, apply := appendRunMacroTooltipEdit(a, actionType)
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

		added = true
		applyParts = append(applyParts, func() error {
			a.Name = strings.TrimSpace(nameEntry.Text)
			a.Tolerance = float32(tolInc.Value)
			a.Blur = blurInc.Value
			applyWait()
			a.RunBranchOnNoFind = runOnNoFind.Value
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
		search.add(actiondisplay.WrapPillStepper(tolInc, actionType))
		sections = append(sections, wrapTooltipSection(search.box))

		wait := newPillRow()
		applyWait := appendWaitTilFoundPills(wait, &a.WaitTilFoundConfig, 100, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Name = strings.TrimSpace(nameEntry.Text)
			a.TargetColor = strings.ToLower(strings.TrimPrefix(strings.TrimSpace(colorEntry.Text), "#"))
			a.ColorTolerance = tolInc.Value
			applyWait()
			return nil
		})

	case *actions.Ocr:
		general := newPillRow()
		nameEntry := addNamePill(general, a.Name, actionType)
		targetEntry := coordEntry(a.Target)
		general.add(actiondisplay.NewEditablePill("Target", targetEntry, actionType))
		sections = append(sections, wrapTooltipSection(general.box))

		wait := newPillRow()
		applyWait := appendWaitTilFoundPills(wait, &a.WaitTilFoundConfig, 0, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Name = strings.TrimSpace(nameEntry.Text)
			a.Target = strings.TrimSpace(targetEntry.Text)
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
		varEntry := coordEntry(a.VariableName)
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
		passToggle := actiondisplay.NewPillToggle("Pass through", a.PassThrough)
		key.add(actiondisplay.WrapPillToggle(passToggle, actionType))
		sections = append(sections, wrapTooltipSection(key.box))

		added = true
		applyParts = append(applyParts, func() error {
			a.Message = messageEntry.Text
			a.PassThrough = passToggle.Value
			return nil
		})

	case *actions.ForEachRow:
		row := newPillRow()
		startEntry := coordEntry(formatCoordValue(a.StartRow))
		endEntry := coordEntry(formatCoordValue(a.EndRow))
		row.add(actiondisplay.NewEditablePill("Start row", startEntry, actionType))
		row.add(actiondisplay.NewEditablePill("End row", endEntry, actionType))
		sections = append(sections, wrapTooltipSection(row.box))
		added = true
		applyParts = append(applyParts, func() error {
			a.StartRow = parseRowBoundValue(startEntry.Text)
			a.EndRow = parseRowBoundValue(endEntry.Text)
			return nil
		})

	case *actions.FocusWindow:
		row := newPillRow()
		titleEntry := coordEntry(a.WindowTitle)
		pathEntry := coordEntry(a.ProcessPath)
		row.add(actiondisplay.NewEditablePill("Title", titleEntry, actionType))
		row.add(actiondisplay.NewEditablePill("App", pathEntry, actionType))
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

	case *actions.Calculate:
		sections = append(sections, appendCalculateTooltipView(a, actionType)...)
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
		added = true

	case *actions.FindPixel:
		general := newPillRow()
		addDisplayPill(general, "Name", a.Name, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

		search := newPillRow()
		addDisplayPill(search, "Color", a.TargetColor, actionType)
		search.add(actiondisplay.NewDisplayPill("Tolerance: "+actions.FormatParamValue(a.ColorTolerance), actionType))
		sections = append(sections, wrapTooltipSection(search.box))

		wait := newPillRow()
		appendWaitTilFoundViewPills(wait, &a.WaitTilFoundConfig, actionType)
		sections = append(sections, wrapTooltipSection(wait.box))
		added = true

	case *actions.Ocr:
		general := newPillRow()
		addDisplayPill(general, "Name", a.Name, actionType)
		addDisplayPill(general, "Target", a.Target, actionType)
		sections = append(sections, wrapTooltipSection(general.box))

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
		addDisplayPill(output, "Variable", a.VariableName, actionType)
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
		row := newPillRow()
		addDisplayPill(row, "Start row", formatCoordValue(a.StartRow), actionType)
		addDisplayPill(row, "End row", formatCoordValue(a.EndRow), actionType)
		sections = append(sections, wrapTooltipSection(row.box))
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
	row.add(actiondisplay.NewDisplayTogglePill("Wait until found", cfg.WaitTilFound, actionType))
	addInlineDisplayPill(row, "Timeout (s)", actions.FormatParamValue(cfg.WaitTilFoundSeconds), actionType)
	addInlineDisplayPill(row, "Interval (ms)", actions.FormatParamValue(cfg.WaitTilFoundIntervalMs), actionType)
}
