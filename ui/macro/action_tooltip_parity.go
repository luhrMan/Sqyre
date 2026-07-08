package macro

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/panicsafe"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"
	"fmt"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

var calculateFunctions = []string{"sqrt", "abs", "round", "floor", "ceil", "trunc", "sin", "cos", "tan", "ln"}
var calculateConstants = []string{"~pi", "~e"}

func showMacroNamePicker(onSelect func(string)) {
	if activeWire.Window == nil || onSelect == nil {
		return
	}
	cur := ""
	if activeWire.MacroContext.CurrentMacro != nil {
		if m := activeWire.MacroContext.CurrentMacro(); m != nil {
			cur = m.Name
		}
	}
	names := repositories.MacroRepo().GetAllKeys()
	if cur != "" {
		names = slices.DeleteFunc(names, func(n string) bool { return n == cur })
	}
	slices.Sort(names)

	filter := custom_widgets.NewFormEntry()
	filter.SetPlaceHolder("Filter macros…")
	filtered := slices.Clone(names)

	var popup *widget.PopUp
	list := widget.NewList(
		func() int { return len(filtered) },
		func() fyne.CanvasObject { return widget.NewLabel("") },
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= 0 && id < len(filtered) {
				obj.(*widget.Label).SetText(filtered[id])
			}
		},
	)
	list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(filtered) {
			return
		}
		onSelect(filtered[id])
		popup.Hide()
	}
	applyFilter := func(q string) {
		q = strings.ToLower(strings.TrimSpace(q))
		filtered = filtered[:0]
		for _, n := range names {
			if q == "" || strings.Contains(strings.ToLower(n), q) {
				filtered = append(filtered, n)
			}
		}
		list.UnselectAll()
		custom_widgets.RefreshListPreservingScroll(list)
	}
	filter.OnChanged = applyFilter

	content := container.NewBorder(filter, nil, nil, nil, list)
	// Non-modal so the margin stays transparent (no solid overlay) and tapping in
	// it dismisses the picker. Leave a macroPickerMarginFrac margin on every window
	// edge and center the picker in the remainder.
	popup = widget.NewPopUp(content, activeWire.Window.Canvas())
	dialogs.AddPopupEscapeClose(popup, activeWire.Window)
	canvasSize := activeWire.Window.Canvas().Size()
	w := canvasSize.Width * (1 - 2*macroPickerMarginFrac)
	h := canvasSize.Height * (1 - 2*macroPickerMarginFrac)
	popup.Resize(fyne.NewSize(w, h))
	popup.ShowAtPosition(fyne.NewPos((canvasSize.Width-w)/2, (canvasSize.Height-h)/2))
}

// macroPickerMarginFrac is the margin left between the run-macro picker and each
// window edge, as a fraction of the window size. 0.2 per edge leaves the picker
// at 60% of the window (a 40% total margin); tapping the margin closes it.
const macroPickerMarginFrac float32 = 0.2

func showWindowPicker(onSelect func(title, path string)) {
	if activeWire.Window == nil || onSelect == nil {
		return
	}
	var allWindows []services.WindowInfo
	filtered := []services.WindowInfo{}

	filter := custom_widgets.NewFormEntry()
	filter.SetPlaceHolder("Filter windows…")

	var popup *widget.PopUp
	list := widget.NewList(
		func() int { return len(filtered) },
		func() fyne.CanvasObject { return widget.NewLabel("") },
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= 0 && id < len(filtered) {
				obj.(*widget.Label).SetText(filtered[id].Label())
			}
		},
	)
	list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(filtered) {
			return
		}
		w := filtered[id]
		onSelect(w.Title, w.ProcessPath)
		popup.Hide()
	}
	applyFilter := func() {
		q := strings.TrimSpace(strings.ToLower(filter.Text))
		filtered = filtered[:0]
		for _, w := range allWindows {
			label := strings.ToLower(w.Label())
			if q == "" || fuzzy.Match(q, label) || fuzzy.Match(q, strings.ToLower(w.Title)) || fuzzy.Match(q, strings.ToLower(w.ProcessPath)) {
				filtered = append(filtered, w)
			}
		}
		list.UnselectAll()
		custom_widgets.RefreshListPreservingScroll(list)
	}
	filter.OnChanged = func(string) { applyFilter() }

	refreshBtn := widget.NewButton("Refresh", func() {
		windows, err := services.ActiveWindows()
		if err != nil {
			allWindows = []services.WindowInfo{{Title: fmt.Sprintf("(error: %v)", err)}}
		} else {
			allWindows = windows
		}
		applyFilter()
	})
	panicsafe.GoSafe(func() {
		windows, err := services.ActiveWindows()
		if err != nil {
			fyne.Do(func() {
				allWindows = []services.WindowInfo{{Title: fmt.Sprintf("(error: %v)", err)}}
				applyFilter()
			})
			return
		}
		fyne.Do(func() {
			allWindows = windows
			applyFilter()
		})
	})

	body := container.NewBorder(filter, refreshBtn, nil, nil, list)
	popup = widget.NewPopUp(body, activeWire.Window.Canvas())
	popup.Resize(fyne.NewSize(520, 400))
	popup.ShowAtPosition(fyne.CurrentApp().Driver().AbsolutePositionForObject(activeWire.Window.Canvas().Content()))
}

func calculateBuilderToolbar(entry *custom_widgets.BorderlessEntry) fyne.CanvasObject {
	operators := []string{"+", "-", "*", "/", "^", "(", ")"}
	opButtons := make([]fyne.CanvasObject, 0, len(operators))
	for _, op := range operators {
		token := op
		b := widget.NewButton(op, func() { entry.InsertAtCursor(token) })
		b.Importance = widget.LowImportance
		opButtons = append(opButtons, b)
	}
	var fxBtn *widget.Button
	fxBtn = widget.NewButton("f(x)", func() {
		items := make([]*fyne.MenuItem, 0, len(calculateFunctions)+len(calculateConstants)+1)
		for _, f := range calculateFunctions {
			fn := f
			items = append(items, fyne.NewMenuItem(fn+"( )", func() { entry.InsertAtCursor(fn + "()") }))
		}
		items = append(items, fyne.NewMenuItemSeparator())
		for _, c := range calculateConstants {
			cst := c
			items = append(items, fyne.NewMenuItem(cst, func() { entry.InsertAtCursor(cst) }))
		}
		c := activeWire.Window.Canvas()
		pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(fxBtn)
		widget.ShowPopUpMenuAtPosition(fyne.NewMenu("", items...), c, pos.Add(fyne.NewPos(0, fxBtn.Size().Height)))
	})
	return container.NewBorder(nil, nil, fxBtn, nil, container.NewHBox(opButtons...))
}

func appendCalculatePreviewRow(exprEntry *custom_widgets.BorderlessEntry, actionType string, owner *actionDisplayTooltipHover) (fyne.CanvasObject, func()) {
	preview := widget.NewLabel("")
	preview.Wrapping = fyne.TextWrapWord
	update := func() {
		prev := preview.Text
		defer func() {
			if preview.Text != prev && owner != nil {
				owner.refreshTooltipLayout()
			}
		}()
		text := strings.TrimSpace(exprEntry.Text)
		if text == "" || activeWire.PreviewExpression == nil {
			preview.SetText("")
			return
		}
		result, err := activeWire.PreviewExpression(text)
		if err != nil {
			preview.SetText("")
			return
		}
		if result == "" {
			preview.SetText("")
			return
		}
		preview.SetText("Preview: " + result)
	}
	exprEntry.OnChanged = func(string) { update() }
	row := newPillRow()
	row.add(actiondisplay.PillChrome(preview, actionType))
	return wrapTooltipSection(row.box), update
}

func findPixelColorDropperButton(colorEntry *custom_widgets.BorderlessEntry, onChanged func()) fyne.CanvasObject {
	if activeWire.ShowRecordingOverlay == nil {
		return nil
	}
	btn := actiondisplay.NewPillIconButton(theme.NewErrorThemedResource(theme.MediaRecordIcon()), func() {
		var dismiss func()
		dismiss = activeWire.ShowRecordingOverlay(nil, func(ev *desktop.MouseEvent) {
			switch ev.Button {
			case desktop.MouseButtonPrimary:
				x, y := screen.Location()
				hex := screen.GetPixelColor(x, y)
				hex = strings.TrimPrefix(strings.ToLower(hex), "#")
				if len(hex) == 8 {
					hex = hex[2:]
				}
				colorEntry.SetText(hex)
				if onChanged != nil {
					onChanged()
				}
				dismiss()
			default:
				dismiss()
			}
		})
	})
	return actiondisplay.PillChrome(btn, "findpixel")
}

func pickerButtonPill(label string, onTap func(), actionType string) fyne.CanvasObject {
	btn := widget.NewButton(label, onTap)
	btn.Importance = widget.MediumImportance
	return actiondisplay.PillChrome(btn, actionType)
}

func macroPickerButton(actionType string, onSelect func(string)) fyne.CanvasObject {
	btn := actiondisplay.NewPillIconButton(theme.ListIcon(), func() {
		showMacroNamePicker(func(name string) {
			onSelect(name)
		})
	})
	return actiondisplay.PillChrome(btn, actionType)
}

func windowPickerButton(actionType string, onSelect func(title, path string)) fyne.CanvasObject {
	return pickerButtonPill("Pick window…", func() {
		showWindowPicker(onSelect)
	}, actionType)
}

const forEachRemoveBtnSize float32 = 28

type forEachSourceEdit struct {
	source    *custom_widgets.BorderlessEntry
	outputVar *custom_widgets.BorderlessVarNameEntry
	isFile    *actiondisplay.PillToggle
	skipBlank *actiondisplay.PillToggle
}

func appendForEachRowTooltipEdit(a *actions.ForEachRow, actionType string, owner *actionDisplayTooltipHover, applyParts []func() error) (fyne.CanvasObject, []func() error) {
	var sections []fyne.CanvasObject

	general := newPillRow()
	nameEntry := addNamePill(general, a.Name, actionType)
	startEntry := coordEntry(formatAnyValue(a.StartRow))
	endEntry := coordEntry(formatAnyValue(a.EndRow))
	general.add(actiondisplay.NewEditablePill("Start row", startEntry, actionType))
	general.add(actiondisplay.NewEditablePill("End row", endEntry, actionType))
	sections = append(sections, wrapTooltipSection(general.box))

	edits := make([]forEachSourceEdit, len(a.Sources))
	for i, s := range a.Sources {
		edits[i] = forEachSourceEdit{
			source:    coordEntry(s.Source),
			outputVar: varNameEntry(s.OutputVar),
			isFile:    actiondisplay.NewPillToggle("Source is file", s.IsFile),
			skipBlank: actiondisplay.NewPillToggle("Skip blank lines", s.SkipBlankLines),
		}
	}
	if len(edits) == 0 {
		edits = append(edits, forEachSourceEdit{
			source:    coordEntry(""),
			outputVar: varNameEntry(""),
			isFile:    actiondisplay.NewPillToggle("Source is file", false),
			skipBlank: actiondisplay.NewPillToggle("Skip blank lines", false),
		})
	}

	sourcesBox := container.NewVBox()
	var rebuildSources func()
	rebuildSources = func() {
		sourcesBox.Objects = nil
		for i := range edits {
			idx := i
			row := newPillRow()
			row.add(actiondisplay.NewEditablePill(fmt.Sprintf("Source %d", idx+1), edits[idx].source, actionType))
			row.add(actiondisplay.NewEditablePill("Output var", edits[idx].outputVar, actionType))
			row.add(actiondisplay.WrapPillToggle(edits[idx].isFile, actionType))
			row.add(actiondisplay.WrapPillToggle(edits[idx].skipBlank, actionType))
			if len(edits) > 1 {
				removeIdx := idx
				removeBtn := widget.NewButtonWithIcon("", theme.DeleteIcon(), func() {
					edits = append(edits[:removeIdx], edits[removeIdx+1:]...)
					rebuildSources()
				})
				removeBtn.Importance = widget.LowImportance
				sized := container.NewGridWrap(fyne.NewSize(forEachRemoveBtnSize, forEachRemoveBtnSize), removeBtn)
				row.add(actiondisplay.PillChrome(sized, actionType))
			}
			sourcesBox.Add(wrapTooltipSection(row.box))
		}
		addBtn := widget.NewButton("Add source column", func() {
			edits = append(edits, forEachSourceEdit{
				source:    coordEntry(""),
				outputVar: varNameEntry(""),
				isFile:    actiondisplay.NewPillToggle("Source is file", false),
				skipBlank: actiondisplay.NewPillToggle("Skip blank lines", false),
			})
			rebuildSources()
		})
		sourcesBox.Add(wrapTooltipSection(container.NewCenter(addBtn)))
		sourcesBox.Refresh()
		if owner != nil {
			owner.relayoutTooltip()
		}
	}
	rebuildSources()
	sections = append(sections, sourcesBox)

	applyParts = append(applyParts, func() error {
		a.Name = strings.TrimSpace(nameEntry.Text)
		a.StartRow = parseRowBoundValue(startEntry.Text)
		a.EndRow = parseRowBoundValue(endEntry.Text)
		sources := make([]actions.ListColumn, 0, len(edits))
		for _, ed := range edits {
			src := strings.TrimSpace(ed.source.Text)
			out := strings.TrimSpace(ed.outputVar.Text)
			if src == "" && out == "" {
				continue
			}
			sources = append(sources, actions.ListColumn{
				Source:         ed.source.Text,
				OutputVar:      out,
				IsFile:         ed.isFile.Value,
				SkipBlankLines: ed.skipBlank.Value,
			})
		}
		a.Sources = sources
		return nil
	})
	return joinTooltipSections(sections...), applyParts
}
