package actiondialog

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

func createSetVariableDialogContent(action *actions.SetVariable) (fyne.CanvasObject, func()) {
	nameEntry := newVarEntry()
	nameEntry.SetText(action.VariableName)
	valueEntry := newVarEntry()
	valueEntry.SetText(fmt.Sprintf("%v", action.Value))

	content := widget.NewForm(
		formHint("Variable Name:", nameEntry, "Name of the macro variable to assign (used as ${Name} in later actions)."),
		formHint("Value:", valueEntry, "New value: number, text, or ${other} expression resolved at runtime."),
	)

	saveFunc := func() {
		action.VariableName = nameEntry.Text
		action.Value = valueEntry.Text // Could be enhanced to parse different types
	}

	return content, saveFunc
}

func createCalculateDialogContent(action *actions.Calculate) (fyne.CanvasObject, func()) {
	exprEntry := newVarEntry()
	exprEntry.SetText(action.Expression)
	varEntry := newVarEntry()
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

func createDataListDialogContent(action *actions.DataList) (fyne.CanvasObject, func()) {
	sourceEntry := newMultiLineVarEntry()
	sourceEntry.SetText(action.Source)
	sourceEntry.SetPlaceHolder("File: path relative to ~/.sqyre/variables/ (e.g. mylist.txt)\nOr paste text directly")
	varEntry := newVarEntry()
	varEntry.SetText(action.OutputVar)
	lengthVarEntry := newVarEntry()
	lengthVarEntry.SetText(action.LengthVar)
	lengthVarEntry.SetPlaceHolder("e.g. lineCount (optional, for Loop)")
	isFileCheck := ttwidget.NewCheck("Source is file path (relative to ~/.sqyre/variables/)", nil)
	isFileCheck.SetChecked(action.IsFile)
	skipBlankCheck := ttwidget.NewCheck("Skip blank lines (exclude from count and iteration)", nil)
	skipBlankCheck.SetChecked(action.SkipBlankLines)

	content := widget.NewForm(
		formHint("Source (file path or text):", sourceEntry, "Either paste lines here or a relative file path under ~/.sqyre/variables/ when the checkbox is on."),
		formHint("", isFileCheck, "When checked, Source is read as a path under ~/.sqyre/variables/. When unchecked, Source is the list content itself."),
		formHint("", skipBlankCheck, "When enabled, empty lines are ignored for line count and iteration indexing."),
		formHint("Output Variable:", varEntry, "Variable holding the current line when iterating with a Data List loop."),
		formHint("Length Variable (optional):", lengthVarEntry, "Optional variable set to the total line count (useful for loops and bounds)."),
	)

	saveFunc := func() {
		action.Source = sourceEntry.Text
		action.OutputVar = varEntry.Text
		action.LengthVar = strings.TrimSpace(lengthVarEntry.Text)
		action.IsFile = isFileCheck.Checked
		action.SkipBlankLines = skipBlankCheck.Checked
	}

	return content, saveFunc
}

func createSaveVariableDialogContent(action *actions.SaveVariable) (fyne.CanvasObject, func()) {
	varEntry := newVarEntry()
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
	// Exclude the currently open macro to prevent infinite recursion
	if cur := currentMacroName(); cur != "" {
		macroNames = slices.DeleteFunc(macroNames, func(name string) bool { return name == cur })
	}
	if len(macroNames) == 0 {
		macroNames = []string{""}
	}
	macroSelect := ttwidget.NewSelect(macroNames, nil)
	if action.MacroName != "" && !slices.Contains(macroNames, action.MacroName) {
		// Macro was deleted or renamed; add current value so it's visible (unless it's the current macro - then clear)
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
		formHint("Macro to run:", macroSelect, "Choose which saved macro executes when this action runs. The current macro cannot call itself (it is omitted from this list)."),
	)

	saveFunc := func() {
		action.MacroName = macroSelect.Selected
	}

	return content, saveFunc
}
