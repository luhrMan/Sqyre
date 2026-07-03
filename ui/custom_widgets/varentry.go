package custom_widgets

import (
	"Sqyre/internal/models"
	"Sqyre/ui/completionentry"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// VarEntry is a widget.Entry for values that may contain ${Variable} references.
// It provides ${ and { partial-name completion, a context-menu variable picker,
// and pill rendering when unfocused.
type VarEntry struct {
	widget.Entry

	// GetVariables returns variable names for completion. Prefer GetVariableDefs when available.
	GetVariables func() []string

	// GetVariableDefs returns rich variable metadata for pickers and pill validation.
	// When set, names are derived from defs and results are cached until the fingerprint changes.
	GetVariableDefs func() []models.VariableDef

	// ChangedFn, if set, is called whenever the text changes. VarEntry owns the
	// embedded Entry.OnChanged to drive variable completion, so callers must use
	// this field instead of setting OnChanged directly.
	ChangedFn func(string)

	// FocusChangedFn is called when the entry gains or loses focus.
	FocusChangedFn func(focused bool)

	completer        *completionentry.PopupCompleter
	feedbackIcon     *ttwidget.Icon
	hasFocus         bool
	hideTextForPills bool
	overlay          *variableRefOverlay
	suppressChanged  bool

	cachedDefFP string
	cachedDefs  []models.VariableDef
	cachedNames []string
	cachedKnown map[string]bool
}

// NewVarEntry creates a single-line entry with variable insertion support.
func NewVarEntry(getVars func() []string) *VarEntry {
	e := &VarEntry{GetVariables: getVars}
	e.ExtendBaseWidget(e)
	e.initCompletion()
	return e
}

// NewVarEntryWithDefs creates a VarEntry backed by variable definitions.
func NewVarEntryWithDefs(getDefs func() []models.VariableDef) *VarEntry {
	e := &VarEntry{GetVariableDefs: getDefs}
	e.ExtendBaseWidget(e)
	e.initCompletion()
	return e
}

// NewMultiLineVarEntry creates a multi-line entry with variable insertion support.
func NewMultiLineVarEntry(getVars func() []string) *VarEntry {
	e := &VarEntry{GetVariables: getVars}
	e.MultiLine = true
	e.Wrapping = fyne.TextWrapWord
	e.ExtendBaseWidget(e)
	e.initCompletion()
	return e
}

// NewMultiLineVarEntryWithDefs creates a multi-line VarEntry backed by definitions.
func NewMultiLineVarEntryWithDefs(getDefs func() []models.VariableDef) *VarEntry {
	e := &VarEntry{GetVariableDefs: getDefs}
	e.MultiLine = true
	e.Wrapping = fyne.TextWrapWord
	e.ExtendBaseWidget(e)
	e.initCompletion()
	return e
}

func (e *VarEntry) initCompletion() {
	e.completer = &completionentry.PopupCompleter{
		Host:       e,
		Entry:      &e.Entry,
		OnSelected: e.completeVarRef,
	}
	e.OnChanged = e.handleChanged
}

func (e *VarEntry) InvalidateVariableCache() {
	e.cachedDefFP = ""
	e.cachedDefs = nil
	e.cachedNames = nil
	e.cachedKnown = nil
}

func (e *VarEntry) variableDefs() []models.VariableDef {
	if e.GetVariableDefs != nil {
		defs := e.GetVariableDefs()
		fp := variableDefsFingerprint(defs)
		if fp == e.cachedDefFP {
			return e.cachedDefs
		}
		e.cachedDefFP = fp
		e.cachedDefs = defs
		e.cachedNames = namesFromDefs(defs)
		e.cachedKnown = knownVariableSet(defs)
		return defs
	}
	if e.GetVariables != nil {
		names := e.GetVariables()
		fp := strings.Join(names, "\x00")
		if fp == e.cachedDefFP {
			return e.cachedDefs
		}
		e.cachedDefFP = fp
		e.cachedNames = names
		defs := make([]models.VariableDef, len(names))
		for i, n := range names {
			defs[i] = models.VariableDef{Name: n}
		}
		e.cachedDefs = defs
		e.cachedKnown = knownVariableSet(defs)
		return defs
	}
	return nil
}

func (e *VarEntry) variableNames() []string {
	e.variableDefs()
	return e.cachedNames
}

func (e *VarEntry) knownVariables() map[string]bool {
	e.variableDefs()
	return e.cachedKnown
}

// SetFeedbackIcon attaches a trailing validation icon on the entry.
func (e *VarEntry) SetFeedbackIcon(icon *ttwidget.Icon) {
	e.feedbackIcon = icon
}

func (e *VarEntry) openVariablePicker() {
	defs := e.variableDefs()
	if len(defs) == 0 {
		return
	}
	ShowVariablePicker(e, defs, e.insertVariable)
}

func (e *VarEntry) CreateRenderer() fyne.WidgetRenderer {
	if e.overlay == nil {
		e.overlay = newVariableRefOverlay()
	}
	base := e.Entry.CreateRenderer()
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
	if e.FocusChangedFn != nil {
		e.FocusChangedFn(true)
	}
	e.Refresh()
}

func (e *VarEntry) FocusLost() {
	e.hasFocus = false
	e.Entry.FocusLost()
	e.syncPillDisplay()
	if e.FocusChangedFn != nil {
		e.FocusChangedFn(false)
	}
	e.Refresh()
}

// HasFocus reports whether this entry currently has keyboard focus.
func (e *VarEntry) HasFocus() bool {
	return e.hasFocus
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

func (e *VarEntry) updateVarCompletion() {
	if e.completer == nil {
		return
	}
	partial, ok := e.varRefContext()
	if !ok {
		e.completer.Hide()
		return
	}
	filtered := filterVarNames(e.variableNames(), partial)
	labels := make([]string, len(filtered))
	for i, n := range filtered {
		labels[i] = n
		for _, d := range e.cachedDefs {
			if d.Name == n {
				labels[i] = VariableDefLabel(d)
				break
			}
		}
	}
	e.completer.ShowLabels(filtered, labels)
}

func (e *VarEntry) varRefContext() (string, bool) {
	line, col := e.cursorLine()
	if partial, ok := partialVarRefAt(line, col, true); ok {
		return partial, true
	}
	return partialVarRefAt(line, col, false)
}

func partialVarRefAt(line []rune, col int, dollar bool) (string, bool) {
	j := col
	for j > 0 && isVarNameRune(line[j-1]) {
		j--
	}
	if dollar {
		if j >= 2 && line[j-1] == '{' && line[j-2] == '$' {
			return string(line[j:col]), true
		}
		return "", false
	}
	if j >= 1 && line[j-1] == '{' {
		if j >= 2 && line[j-2] == '$' {
			return "", false
		}
		return string(line[j:col]), true
	}
	return "", false
}

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
	col := min(e.CursorColumn, len(line))
	start, ok := varRefReplaceStart(line, col)
	if !ok {
		return
	}
	replacement := []rune("${" + name + "}")
	newLine := append(append(append([]rune{}, line[:start]...), replacement...), line[col:]...)
	lines[row] = string(newLine)
	e.SetText(joinLines(lines))
	e.CursorRow = row
	e.CursorColumn = start + len(replacement)
	e.Refresh()
}

func varRefReplaceStart(line []rune, col int) (int, bool) {
	if start, ok := varRefStartAt(line, col, true); ok {
		return start, true
	}
	return varRefStartAt(line, col, false)
}

func varRefStartAt(line []rune, col int, dollar bool) (int, bool) {
	j := col
	for j > 0 && isVarNameRune(line[j-1]) {
		j--
	}
	if dollar {
		if j >= 2 && line[j-1] == '{' && line[j-2] == '$' {
			return j - 2, true
		}
		return 0, false
	}
	if j >= 1 && line[j-1] == '{' {
		if j >= 2 && line[j-2] == '$' {
			return 0, false
		}
		return j - 1, true
	}
	return 0, false
}

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
	col := min(e.CursorColumn, len(line))
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

	if len(e.variableDefs()) > 0 {
		menuItems = append(menuItems, fyne.NewMenuItemSeparator())
		menuItems = append(menuItems, fyne.NewMenuItem("Insert Variable…", func() {
			e.openVariablePicker()
		}))
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
// current selection if there is one.
func (e *VarEntry) InsertAtCursor(text string) {
	if text == "" {
		return
	}

	if sel := e.SelectedText(); sel != "" {
		e.replaceSelection(text)
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

func (e *VarEntry) replaceSelection(replacement string) {
	off := e.CursorTextOffset()
	selRunes := len([]rune(e.SelectedText()))
	start := max(off-selRunes, 0)
	runes := []rune(e.Text)
	end := min(off, len(runes))
	newRunes := append(append(append([]rune{}, runes[:start]...), []rune(replacement)...), runes[end:]...)
	newText := string(newRunes)
	e.setTextFromEdit(newText)
	newOff := start + len([]rune(replacement))
	row, col := textOffsetToCursor(newText, e.MultiLine, newOff)
	e.CursorRow = row
	e.CursorColumn = col
}

func textOffsetToCursor(text string, multiLine bool, offset int) (row, col int) {
	if !multiLine {
		return 0, offset
	}
	pos := 0
	for i, line := range splitLines(text) {
		lineRunes := []rune(line)
		lineEnd := pos + len(lineRunes)
		if offset <= lineEnd {
			return i, offset - pos
		}
		pos = lineEnd + 1
	}
	if len(text) == 0 {
		return 0, 0
	}
	lines := splitLines(text)
	return len(lines) - 1, len([]rune(lines[len(lines)-1]))
}

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
	case *Incrementer:
		return strconv.Itoa(e.Value)
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
	case *Incrementer:
		if v, err := strconv.Atoi(strings.TrimSpace(text)); err == nil {
			e.SetValue(v)
		}
	}
}

func splitLines(s string) []string {
	return strings.Split(s, "\n")
}

func joinLines(lines []string) string {
	return strings.Join(lines, "\n")
}
