package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// VarEntry is a widget.Entry that adds a "Variables" submenu to the
// right-click context menu. Selecting a variable inserts ${Name} at
// the current cursor position (or replaces the selection).
type VarEntry struct {
	widget.Entry

	// GetVariables is called each time the context menu opens.
	// It should return the current list of available variable names.
	GetVariables func() []string
}

// NewVarEntry creates a single-line entry with a Variables context menu.
func NewVarEntry(getVars func() []string) *VarEntry {
	e := &VarEntry{GetVariables: getVars}
	e.ExtendBaseWidget(e)
	return e
}

// NewMultiLineVarEntry creates a multi-line entry with a Variables context menu.
func NewMultiLineVarEntry(getVars func() []string) *VarEntry {
	e := &VarEntry{GetVariables: getVars}
	e.MultiLine = true
	e.Wrapping = fyne.TextWrapWord
	e.ExtendBaseWidget(e)
	return e
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
	ref := "${" + name + "}"

	if e.SelectedText() != "" {
		clipboard := fyne.CurrentApp().Clipboard()
		clipboard.SetContent(ref)
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
			lineRunes = append(lineRunes[:cur], append([]rune(ref), lineRunes[cur:]...)...)
			lines[row] = string(lineRunes)
			e.SetText(joinLines(lines))
			e.CursorRow = row
			e.CursorColumn = cur + len([]rune(ref))
		} else {
			e.SetText(e.Text + ref)
		}
	} else {
		runes := []rune(e.Text)
		if cur > len(runes) {
			cur = len(runes)
		}
		runes = append(runes[:cur], append([]rune(ref), runes[cur:]...)...)
		e.SetText(string(runes))
		e.CursorColumn = cur + len([]rune(ref))
	}
	e.Refresh()
}

// EntryText returns the Text field from a *widget.Entry or *VarEntry.
func EntryText(w fyne.CanvasObject) string {
	switch e := w.(type) {
	case *widget.Entry:
		return e.Text
	case *VarEntry:
		return e.Text
	}
	return ""
}

// SetEntryText calls SetText on a *widget.Entry or *VarEntry.
func SetEntryText(w fyne.CanvasObject, text string) {
	switch e := w.(type) {
	case *widget.Entry:
		e.SetText(text)
	case *VarEntry:
		e.SetText(text)
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
