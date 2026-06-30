package custom_widgets

import (
	"Sqyre/ui/completionentry"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// VarEntry is a widget.Entry for values that may contain ${Variable} references.
// It provides ${ partial-name completion, a Variables right-click submenu, a + button
// to pick a variable, and pill rendering when unfocused.
type VarEntry struct {
	widget.Entry

	// GetVariables is called each time the context menu opens.
	// It should return the current list of available variable names.
	GetVariables func() []string

	// ChangedFn, if set, is called whenever the text changes. VarEntry owns the
	// embedded Entry.OnChanged to drive variable completion, so callers must use
	// this field instead of setting OnChanged directly.
	ChangedFn func(string)

	completer        *completionentry.PopupCompleter
	insert           *ttwidget.Button
	feedbackIcon     *ttwidget.Icon
	hasFocus         bool
	hideTextForPills bool
	overlay          *variableRefOverlay
	// suppressChanged blocks completion and ChangedFn while SetText loads values programmatically.
	suppressChanged bool
}

// NewVarEntry creates a single-line entry with a Variables context menu.
func NewVarEntry(getVars func() []string) *VarEntry {
	e := &VarEntry{GetVariables: getVars}
	e.ExtendBaseWidget(e)
	e.initCompletion()
	return e
}

// NewMultiLineVarEntry creates a multi-line entry with a Variables context menu.
func NewMultiLineVarEntry(getVars func() []string) *VarEntry {
	e := &VarEntry{GetVariables: getVars}
	e.MultiLine = true
	e.Wrapping = fyne.TextWrapWord
	e.ExtendBaseWidget(e)
	e.initCompletion()
	return e
}

// initCompletion wires the variable-reference completion popup and the
// internal OnChanged dispatcher. It must be called after ExtendBaseWidget.
func (e *VarEntry) initCompletion() {
	e.completer = &completionentry.PopupCompleter{
		Host:       e,
		Entry:      &e.Entry,
		OnSelected: e.completeVarRef,
	}
	e.OnChanged = e.handleChanged
	e.ensureInsertButton()
}

// SetFeedbackIcon attaches a trailing validation icon rendered immediately before the insert button.
func (e *VarEntry) SetFeedbackIcon(icon *ttwidget.Icon) {
	e.feedbackIcon = icon
}

func (e *VarEntry) ensureInsertButton() {
	if e.insert != nil {
		return
	}
	e.insert = ttwidget.NewButtonWithIcon("", theme.ContentAddIcon(), func() {
		e.showVariableMenu()
	})
	e.insert.Importance = widget.LowImportance
	e.insert.SetToolTip("Insert variable reference (${name})")
	e.UpdateInsertButton()
}

// UpdateInsertButton enables the insert button when variables are available.
func (e *VarEntry) UpdateInsertButton() {
	if e.insert == nil {
		return
	}
	if e.GetVariables != nil && len(e.GetVariables()) > 0 {
		e.insert.Enable()
		return
	}
	e.insert.Disable()
}

func (e *VarEntry) showVariableMenu() {
	e.UpdateInsertButton()
	if e.GetVariables == nil {
		return
	}
	vars := e.GetVariables()
	if len(vars) == 0 {
		return
	}
	items := make([]*fyne.MenuItem, len(vars))
	for i, v := range vars {
		name := v
		items[i] = fyne.NewMenuItem(name, func() {
			e.insertVariable(name)
		})
	}
	c := fyne.CurrentApp().Driver().CanvasForObject(e)
	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(e)
	widget.ShowPopUpMenuAtPosition(fyne.NewMenu("", items...), c, pos.Add(fyne.NewPos(0, e.Size().Height)))
}

func (e *VarEntry) CreateRenderer() fyne.WidgetRenderer {
	if e.overlay == nil {
		e.overlay = newVariableRefOverlay()
	}
	base := e.Entry.CreateRenderer()
	// Entry.CreateRenderer resets impl to *Entry; restore *VarEntry for focus/theme routing.
	e.ExtendBaseWidget(e)
	return &varEntryRendererWrap{
		inner:   base,
		overlay: e.overlay,
		entry:   e,
	}
}

func (e *VarEntry) FocusGained() {
	e.hasFocus = true
	e.Entry.FocusGained()
	e.syncPillDisplay()
	e.Refresh()
}

func (e *VarEntry) FocusLost() {
	e.hasFocus = false
	e.Entry.FocusLost()
	e.syncPillDisplay()
	e.Refresh()
}

func (e *VarEntry) SetText(text string) {
	if e.completer != nil {
		e.completer.HideWithoutRefocus()
	}
	e.suppressChanged = true
	e.Entry.SetText(text)
	e.suppressChanged = false
	e.syncPillDisplay()
}

func (e *VarEntry) shouldShowPills() bool {
	if e.Text == "" || !textContainsVarRef(e.Text) {
		return false
	}
	return !e.hasFocus
}

func (e *VarEntry) syncPillDisplay() {
	e.hideTextForPills = e.shouldShowPills()
}

func (e *VarEntry) handleChanged(s string) {
	if e.suppressChanged {
		return
	}
	e.updateVarCompletion()
	e.syncPillDisplay()
	if e.ChangedFn != nil {
		e.ChangedFn(s)
	}
}

// updateVarCompletion shows or hides the variable completion popup based on
// whether "${" (optionally followed by a partial name) sits behind the cursor.
func (e *VarEntry) updateVarCompletion() {
	if e.completer == nil || e.GetVariables == nil {
		return
	}
	partial, ok := e.varRefContext()
	if !ok {
		e.completer.Hide()
		return
	}
	filtered := filterVarNames(e.GetVariables(), partial)
	e.completer.Show(filtered)
}

// varRefContext reports the partial variable name being typed when the text
// directly behind the cursor is "${" optionally followed by name characters.
func (e *VarEntry) varRefContext() (string, bool) {
	line, col := e.cursorLine()
	j := col
	for j > 0 && isVarNameRune(line[j-1]) {
		j--
	}
	if j >= 2 && line[j-1] == '{' && line[j-2] == '$' {
		return string(line[j:col]), true
	}
	return "", false
}

// completeVarRef replaces the "${partial" behind the cursor with "${name}" and
// positions the cursor after the closing brace.
func (e *VarEntry) completeVarRef(name string) {
	lines := splitLines(e.Text)
	row := e.CursorRow
	if e.MultiLine {
		if row < 0 || row >= len(lines) {
			return
		}
	} else {
		row = 0
	}
	line := []rune(lines[row])
	col := e.CursorColumn
	if col > len(line) {
		col = len(line)
	}
	j := col
	for j > 0 && isVarNameRune(line[j-1]) {
		j--
	}
	if !(j >= 2 && line[j-1] == '{' && line[j-2] == '$') {
		return
	}
	start := j - 2
	replacement := []rune("${" + name + "}")
	newLine := append(append(append([]rune{}, line[:start]...), replacement...), line[col:]...)
	lines[row] = string(newLine)
	e.SetText(joinLines(lines))
	e.CursorRow = row
	e.CursorColumn = start + len(replacement)
	e.Refresh()
}

// cursorLine returns the current line (as runes) and the cursor column within it.
func (e *VarEntry) cursorLine() ([]rune, int) {
	lines := splitLines(e.Text)
	row := 0
	if e.MultiLine {
		row = e.CursorRow
	}
	if row < 0 || row >= len(lines) {
		return nil, 0
	}
	line := []rune(lines[row])
	col := e.CursorColumn
	if col > len(line) {
		col = len(line)
	}
	return line, col
}

func isVarNameRune(r rune) bool {
	return r == '_' ||
		(r >= 'a' && r <= 'z') ||
		(r >= 'A' && r <= 'Z') ||
		(r >= '0' && r <= '9')
}

func filterVarNames(names []string, partial string) []string {
	p := strings.ToLower(strings.TrimSpace(partial))
	out := make([]string, 0, len(names))
	for _, n := range names {
		if p == "" || strings.HasPrefix(strings.ToLower(n), p) {
			out = append(out, n)
		}
	}
	return out
}

// TappedSecondary shows the standard context menu plus a Variables submenu.
func (e *VarEntry) TappedSecondary(pe *fyne.PointEvent) {
	if e.Disabled() && e.Password {
		return
	}

	clipboard := fyne.CurrentApp().Clipboard()

	cutItem := fyne.NewMenuItem("Cut", func() {
		e.TypedShortcut(&fyne.ShortcutCut{Clipboard: clipboard})
	})
	copyItem := fyne.NewMenuItem("Copy", func() {
		e.TypedShortcut(&fyne.ShortcutCopy{Clipboard: clipboard})
	})
	pasteItem := fyne.NewMenuItem("Paste", func() {
		e.TypedShortcut(&fyne.ShortcutPaste{Clipboard: clipboard})
	})
	selectAllItem := fyne.NewMenuItem("Select All", func() {
		e.TypedShortcut(&fyne.ShortcutSelectAll{})
	})

	menuItems := make([]*fyne.MenuItem, 0, 8)
	if e.Disabled() {
		menuItems = append(menuItems, copyItem, selectAllItem)
	} else if e.Password {
		menuItems = append(menuItems, pasteItem, selectAllItem)
	} else {
		menuItems = append(menuItems, cutItem, copyItem, pasteItem, selectAllItem)
	}

	if e.GetVariables != nil {
		vars := e.GetVariables()
		if len(vars) > 0 {
			varChildren := make([]*fyne.MenuItem, len(vars))
			for i, v := range vars {
				varName := v
				varChildren[i] = fyne.NewMenuItem(varName, func() {
					e.insertVariable(varName)
				})
			}
			varsItem := fyne.NewMenuItem("Variables", nil)
			varsItem.ChildMenu = fyne.NewMenu("", varChildren...)
			menuItems = append(menuItems, fyne.NewMenuItemSeparator(), varsItem)
		}
	}

	driver := fyne.CurrentApp().Driver()
	entryPos := driver.AbsolutePositionForObject(e)
	popUpPos := entryPos.Add(pe.Position)
	c := driver.CanvasForObject(e)
	widget.ShowPopUpMenuAtPosition(fyne.NewMenu("", menuItems...), c, popUpPos)
}

func (e *VarEntry) insertVariable(name string) {
	e.InsertAtCursor("${" + name + "}")
}

// InsertAtCursor inserts text at the current cursor position, replacing the
// current selection if there is one. Used to build expressions by inserting
// variable references, operators, and functions.
func (e *VarEntry) InsertAtCursor(text string) {
	if text == "" {
		return
	}

	if e.SelectedText() != "" {
		clipboard := fyne.CurrentApp().Clipboard()
		clipboard.SetContent(text)
		e.TypedShortcut(&fyne.ShortcutPaste{Clipboard: clipboard})
		return
	}

	cur := e.CursorColumn
	if e.MultiLine {
		row := e.CursorRow
		lines := splitLines(e.Text)
		if row >= 0 && row < len(lines) {
			lineRunes := []rune(lines[row])
			if cur > len(lineRunes) {
				cur = len(lineRunes)
			}
			lineRunes = append(lineRunes[:cur], append([]rune(text), lineRunes[cur:]...)...)
			lines[row] = string(lineRunes)
			e.setTextFromEdit(joinLines(lines))
			e.CursorRow = row
			e.CursorColumn = cur + len([]rune(text))
		} else {
			e.setTextFromEdit(e.Text + text)
		}
	} else {
		runes := []rune(e.Text)
		if cur > len(runes) {
			cur = len(runes)
		}
		runes = append(runes[:cur], append([]rune(text), runes[cur:]...)...)
		e.setTextFromEdit(string(runes))
		e.CursorColumn = cur + len([]rune(text))
	}
	e.Refresh()
}

// setTextFromEdit applies text from user-driven edits after the cursor position is known.
func (e *VarEntry) setTextFromEdit(text string) {
	e.suppressChanged = true
	e.Entry.SetText(text)
	e.suppressChanged = false
	e.handleChanged(text)
}

// EntryText returns the Text field from supported entry widgets.
func EntryText(w fyne.CanvasObject) string {
	switch e := w.(type) {
	case *widget.Entry:
		return e.Text
	case *VarEntry:
		return e.Text
	case *VarEntryField:
		return e.Entry.Text
	}
	return ""
}

// SetEntryText calls SetText on supported entry types.
func SetEntryText(w fyne.CanvasObject, text string) {
	switch e := w.(type) {
	case *widget.Entry:
		e.SetText(text)
	case *VarEntry:
		e.SetText(text)
	case *VarEntryField:
		e.Entry.SetText(text)
		e.Revalidate()
	}
}

func splitLines(s string) []string {
	lines := []string{}
	start := 0
	for i, ch := range s {
		if ch == '\n' {
			lines = append(lines, s[start:i])
			start = i + 1
		}
	}
	lines = append(lines, s[start:])
	return lines
}

func joinLines(lines []string) string {
	if len(lines) == 0 {
		return ""
	}
	result := lines[0]
	for i := 1; i < len(lines); i++ {
		result += "\n" + lines[i]
	}
	return result
}
