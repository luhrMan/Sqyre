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
	valueEntry := newVarEntry()
	valueEntry.SetText(fmt.Sprintf("%v", action.Value))

	content := widget.NewForm(
		formHint("Variable Name:", nameEntry, "Name of the macro variable to assign (used as ${Name} in later actions)."),
		formHint("Value:", valueEntry, "New value: number, text, or ${other} expression resolved at runtime."),
	)

	saveFunc := func() {
		action.VariableName = nameEntry.Text
		action.Value = valueEntry.Text
	}

	return content, saveFunc
}

func createCalculateDialogContent(action *actions.Calculate) (fyne.CanvasObject, func()) {
	exprEntry := newVarEntry()
	exprEntry.SetText(action.Expression)
	varEntry := newVarNameEntry()
	varEntry.SetText(action.OutputVar)

	content := widget.NewForm(
		formHint("Expression:", exprEntry, "Arithmetic or expression to evaluate (macro variables as ${name}). Result is stored in the output variable."),
		formHint("Output Variable:", varEntry, "Variable that receives the calculated result."),
	)

	saveFunc := func() {
		action.Expression = exprEntry.Text
		action.OutputVar = varEntry.Text
	}

	return content, saveFunc
}

type sourceRowWidgets struct {
	source    *custom_widgets.VarRefEntry
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

func forEachRowSourceField(source *custom_widgets.VarRefEntry) fyne.CanvasObject {
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
	destEntry := newVarEntry()
	destEntry.SetText(action.Destination)
	destEntry.SetPlaceHolder("~/.sqyre/variables/... or 'clipboard'")
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
		action.Destination = destEntry.Text
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
