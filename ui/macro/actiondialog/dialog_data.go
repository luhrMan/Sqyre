package actiondialog

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

const (
	forEachRowSourceEntryMinHeight = float32(120)
	forEachRowSourcesScrollMinHeight = float32(320)
	forEachRowDialogWidth            = float32(720)
	forEachRowDialogHeight           = float32(620)
)

func createSetVariableDialogContent(action *actions.SetVariable) (fyne.CanvasObject, func()) {
	nameEntry := newVarNameEntry()
	nameEntry.SetText(action.VariableName)
	valueEntry := newValidatedVarEntry(validateSetVariableValue)
	valueEntry.Entry.SetText(fmt.Sprintf("%v", action.Value))

	content := widget.NewForm(
		formHint("Variable Name:", nameEntry, "Name of the macro variable to assign (used as ${Name} in later actions)."),
		formHint("Value:", valueEntry, "New value: number, text, or ${other} expression resolved at runtime."),
	)

	saveFunc := func() {
		action.VariableName = nameEntry.Text
		action.Value = valueEntry.Entry.Text
	}

	return content, saveFunc
}

// calculateFunctions are the functions supported by the expression evaluator.
// Each inserts a call template into the expression entry.
var calculateFunctions = []string{"sqrt", "abs", "round", "floor", "ceil", "trunc", "sin", "cos", "tan", "ln"}

// calculateConstants are the constants supported by the evaluator.
var calculateConstants = []string{"~pi", "~e"}

func createCalculateDialogContent(action *actions.Calculate) (fyne.CanvasObject, func()) {
	exprEntry := newValidatedMultiLineVarEntry(validateCalculateExpression)
	exprEntry.Entry.SetText(action.Expression)
	varEntry := newVarNameEntry()
	varEntry.SetText(action.OutputVar)

	preview := widget.NewLabel("")
	preview.Wrapping = fyne.TextWrapWord
	updatePreview := func() {
		if strings.TrimSpace(exprEntry.Entry.Text) == "" {
			preview.SetText("")
			return
		}
		if !exprEntry.Valid() {
			preview.SetText("")
			return
		}
		result, err := previewExpression(exprEntry.Entry.Text)
		if err != nil {
			preview.SetText("")
			return
		}
		preview.SetText("Preview: " + result)
	}
	exprEntry.OnChanged = func(string) { updatePreview() }
	exprEntry.SetOnValidationChanged(updatePreview)

	toolbar := calculateBuilderToolbar(exprEntry.Entry)

	exprField := container.NewGridWrap(fyne.NewSize(560, 96), exprEntry)

	content := container.NewVBox(
		widget.NewLabelWithStyle("Expression", fyne.TextAlignLeading, fyne.TextStyle{Bold: true}),
		toolbar,
		exprField,
		preview,
		widget.NewSeparator(),
		widget.NewForm(
			formHint("Output Variable:", varEntry, "Variable that receives the calculated result."),
		),
	)

	updatePreview()

	saveFunc := func() {
		action.Expression = exprEntry.Entry.Text
		action.OutputVar = varEntry.Text
	}

	return content, saveFunc
}

// calculateBuilderToolbar builds the insert-variable, operator, and function
// controls that write into the expression entry at the cursor.
func calculateBuilderToolbar(exprEntry *custom_widgets.VarEntry) fyne.CanvasObject {
	var varBtn *widget.Button
	varBtn = widget.NewButtonWithIcon("Variable", theme.ContentAddIcon(), func() {
		names := macroVarNames()
		if len(names) == 0 {
			return
		}
		items := make([]*fyne.MenuItem, len(names))
		for i, n := range names {
			name := n
			items[i] = fyne.NewMenuItem(name, func() {
				exprEntry.InsertAtCursor("${" + name + "}")
			})
		}
		popUpMenuBelow(fyne.NewMenu("", items...), varBtn)
	})

	operators := []string{"+", "-", "*", "/", "^", "(", ")"}
	opButtons := make([]fyne.CanvasObject, 0, len(operators))
	for _, op := range operators {
		token := op
		b := widget.NewButton(op, func() { exprEntry.InsertAtCursor(token) })
		b.Importance = widget.LowImportance
		opButtons = append(opButtons, b)
	}

	var fxBtn *widget.Button
	fxBtn = widget.NewButton("f(x)", func() {
		items := make([]*fyne.MenuItem, 0, len(calculateFunctions)+len(calculateConstants)+1)
		for _, f := range calculateFunctions {
			fn := f
			items = append(items, fyne.NewMenuItem(fn+"( )", func() {
				exprEntry.InsertAtCursor(fn + "()")
			}))
		}
		items = append(items, fyne.NewMenuItemSeparator())
		for _, c := range calculateConstants {
			cst := c
			items = append(items, fyne.NewMenuItem(cst, func() {
				exprEntry.InsertAtCursor(cst)
			}))
		}
		popUpMenuBelow(fyne.NewMenu("", items...), fxBtn)
	})

	left := container.NewHBox(varBtn, fxBtn)
	ops := container.NewHBox(opButtons...)
	return container.NewBorder(nil, nil, left, nil, ops)
}

func popUpMenuBelow(menu *fyne.Menu, anchor fyne.CanvasObject) {
	driver := fyne.CurrentApp().Driver()
	c := driver.CanvasForObject(anchor)
	if c == nil {
		return
	}
	pos := driver.AbsolutePositionForObject(anchor)
	widget.ShowPopUpMenuAtPosition(menu, c, pos.Add(fyne.NewPos(0, anchor.Size().Height)))
}

type sourceRowWidgets struct {
	source    *custom_widgets.VarEntry
	outputVar *custom_widgets.VarNameEntry
	isFile    *ttwidget.Check
	skipBlank *ttwidget.Check
}

func newSourceRowWidgets() sourceRowWidgets {
	source := newMultiLineVarEntry()
	source.SetPlaceHolder("File path or one value per line")
	return sourceRowWidgets{
		source:    source,
		outputVar: newVarNameEntry(),
		isFile:    ttwidget.NewCheck("Source is file path", nil),
		skipBlank: ttwidget.NewCheck("Skip blank lines", nil),
	}
}

func forEachRowSourceField(source *custom_widgets.VarEntry) fyne.CanvasObject {
	return container.NewGridWrap(
		fyne.NewSize(forEachRowDialogWidth-200, forEachRowSourceEntryMinHeight),
		source,
	)
}

func createForEachRowDialogContent(action *actions.ForEachRow) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)

	rows := make([]sourceRowWidgets, len(action.Sources))
	for i, s := range action.Sources {
		rows[i] = newSourceRowWidgets()
		rows[i].source.SetText(s.Source)
		rows[i].outputVar.SetText(s.OutputVar)
		rows[i].isFile.SetChecked(s.IsFile)
		rows[i].skipBlank.SetChecked(s.SkipBlankLines)
	}
	if len(rows) == 0 {
		rows = append(rows, newSourceRowWidgets())
	}

	rowsBox := container.NewVBox()
	var rebuild func()
	rebuild = func() {
		rowsBox.Objects = nil
		for i := range rows {
			idx := i
			row := rows[idx]
			removeBtn := widget.NewButton("Remove", func() {
				if len(rows) <= 1 {
					return
				}
				rows = append(rows[:idx], rows[idx+1:]...)
				rebuild()
			})
			if len(rows) <= 1 {
				removeBtn.Disable()
			}
			header := container.NewHBox(
				widget.NewLabelWithStyle("Source column", fyne.TextAlignLeading, fyne.TextStyle{Bold: true}),
				widget.NewLabel("(first column sets row count)"),
			)
			rowForm := widget.NewForm(
				formHint("Source:", forEachRowSourceField(row.source), "File path under ~/.sqyre/variables/ or pasted line-separated text."),
				formHint("", row.isFile, "Read Source as a file path when checked."),
				formHint("", row.skipBlank, "Ignore empty lines in this column."),
				formHint("Output Variable:", row.outputVar, "Macro variable for the current row value. Sub-actions also get ${Row} and ${RowCount}."),
			)
			rowsBox.Add(container.NewVBox(
				header,
				rowForm,
				removeBtn,
				widget.NewSeparator(),
			))
		}
	}
	rebuild()

	addBtn := widget.NewButton("Add source column", func() {
		rows = append(rows, newSourceRowWidgets())
		rebuild()
	})

	sourceScroll := container.NewVScroll(rowsBox)
	sourceScroll.SetMinSize(fyne.NewSize(forEachRowDialogWidth-120, forEachRowSourcesScrollMinHeight))

	content := widget.NewForm(
		formHint("Name:", nameEntry, "Label for this iterator in the tree."),
	)
	content.Append("Sources", container.NewBorder(
		addBtn,
		nil, nil, nil,
		sourceScroll,
	))

	saveFunc := func() {
		action.Name = nameEntry.Text
		sources := make([]actions.ListColumn, 0, len(rows))
		for _, row := range rows {
			src := strings.TrimSpace(row.source.Text)
			out := strings.TrimSpace(row.outputVar.Text)
			if src == "" && out == "" {
				continue
			}
			sources = append(sources, actions.ListColumn{
				Source:         row.source.Text,
				OutputVar:      out,
				IsFile:         row.isFile.Checked,
				SkipBlankLines: row.skipBlank.Checked,
			})
		}
		action.Sources = sources
	}

	return content, saveFunc
}

func createSaveVariableDialogContent(action *actions.SaveVariable) (fyne.CanvasObject, func()) {
	varEntry := newVarNameEntry()
	varEntry.SetText(action.VariableName)
	destEntry := newValidatedVarEntry(validateVariableReferences)
	destEntry.Entry.SetText(action.Destination)
	destEntry.Entry.SetPlaceHolder("~/.sqyre/variables/... or 'clipboard'")
	appendCheck := ttwidget.NewCheck("Append to file", nil)
	appendCheck.SetChecked(action.Append)
	appendNewlineCheck := ttwidget.NewCheck("New line with every append", nil)
	appendNewlineCheck.SetChecked(action.AppendNewline)

	content := widget.NewForm(
		formHint("Variable Name:", varEntry, "Variable whose value will be written out."),
		formHint("Destination (~/.sqyre/variables/... or 'clipboard'):", destEntry, "Relative path under ~/.sqyre/variables/, or the word clipboard to copy to the system clipboard."),
		formHint("", appendCheck, "When checked, new content is appended to the file instead of overwriting. Ignored for clipboard."),
		formHint("", appendNewlineCheck, "When appending, add a newline before each write so values stay on separate lines."),
	)

	saveFunc := func() {
		action.VariableName = varEntry.Text
		action.Destination = destEntry.Entry.Text
		action.Append = appendCheck.Checked
		action.AppendNewline = appendNewlineCheck.Checked
	}

	return content, saveFunc
}

func createRunMacroDialogContent(action *actions.RunMacro) (fyne.CanvasObject, func()) {
	macroNames := repositories.MacroRepo().GetAllKeys()
	if cur := currentMacroName(); cur != "" {
		macroNames = slices.DeleteFunc(macroNames, func(name string) bool { return name == cur })
	}
	if len(macroNames) == 0 {
		macroNames = []string{""}
	}
	macroSelect := ttwidget.NewSelect(macroNames, nil)
	if action.MacroName != "" && !slices.Contains(macroNames, action.MacroName) {
		if cur := currentMacroName(); action.MacroName != "" && action.MacroName == cur && cur != "" {
			macroNames = append([]string{""}, macroNames...)
			macroSelect.Options = macroNames
			macroSelect.SetSelected("")
		} else {
			macroSelect.Options = append([]string{action.MacroName}, macroNames...)
			macroSelect.SetSelected(action.MacroName)
		}
	} else {
		macroSelect.SetSelected(action.MacroName)
	}

	content := widget.NewForm(
		formHint("Macro to run:", macroSelect, "Choose which saved macro executes when this action runs. The current macro cannot call itself (it is omitted from this list). The called macro uses its own variable store; variables are not shared with the caller."),
	)

	saveFunc := func() {
		action.MacroName = macroSelect.Selected
	}

	return content, saveFunc
}
