package custom_widgets

import (
	"Sqyre/internal/models"
	"Sqyre/ui/completionentry"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// VarNameEntry is for fields that define a variable name (not ${references}).
// It offers prefix completion, a searchable variable picker (+), and a context-menu picker.
type VarNameEntry struct {
	completionentry.CompletionEntry

	getNames        func() []string
	GetVariableDefs func() []models.VariableDef

	insert *ttwidget.Button

	cachedDefFP string
	cachedDefs  []models.VariableDef
	cachedNames []string
	shownDefs   []models.VariableDef
}

// NewVarNameEntry creates an entry for naming macro variables.
func NewVarNameEntry(getNames func() []string) *VarNameEntry {
	e := &VarNameEntry{getNames: getNames}
	e.init()
	return e
}

// NewVarNameEntryWithDefs creates an entry backed by variable definitions.
func NewVarNameEntryWithDefs(getDefs func() []models.VariableDef) *VarNameEntry {
	e := &VarNameEntry{GetVariableDefs: getDefs}
	e.init()
	return e
}

func (e *VarNameEntry) init() {
	e.ExtendBaseWidget(e)
	e.OnChanged = func(_ string) {
		e.refreshOptions()
	}
	e.initRichCompletion()
	e.ensureInsertButton()
}

func (e *VarNameEntry) initRichCompletion() {
	e.CustomCreate = func() fyne.CanvasObject {
		return container.NewVBox(
			widget.NewLabel(""),
			widget.NewLabelWithStyle("", fyne.TextAlignLeading, fyne.TextStyle{Italic: true}),
		)
	}
	e.CustomUpdate = func(id widget.ListItemID, obj fyne.CanvasObject) {
		box := obj.(*fyne.Container)
		nameLbl := box.Objects[0].(*widget.Label)
		metaLbl := box.Objects[1].(*widget.Label)
		if int(id) >= len(e.shownDefs) {
			return
		}
		d := e.shownDefs[id]
		nameLbl.SetText(d.Name)
		meta := VariableDefLabel(d)
		if meta == d.Name {
			metaLbl.SetText("")
			metaLbl.Hide()
		} else {
			metaLbl.SetText(strings.TrimPrefix(meta, d.Name+" · "))
			metaLbl.Show()
		}
	}
}

func (e *VarNameEntry) variableDefs() []models.VariableDef {
	if e.GetVariableDefs != nil {
		defs := e.GetVariableDefs()
		fp := variableDefsFingerprint(defs)
		if fp == e.cachedDefFP {
			return e.cachedDefs
		}
		e.cachedDefFP = fp
		e.cachedDefs = defs
		e.cachedNames = namesFromDefs(defs)
		return defs
	}
	if e.getNames != nil {
		names := e.getNames()
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
		return defs
	}
	return nil
}

func (e *VarNameEntry) InvalidateVariableCache() {
	e.cachedDefFP = ""
	e.cachedDefs = nil
	e.cachedNames = nil
}

func (e *VarNameEntry) ensureInsertButton() {
	if e.insert != nil {
		return
	}
	e.insert = ttwidget.NewButtonWithIcon("", theme.ContentAddIcon(), func() {
		e.openVariablePicker()
	})
	e.insert.Importance = widget.LowImportance
	e.insert.SetToolTip("Pick an existing variable name")
	e.UpdateInsertButton()
}

func (e *VarNameEntry) UpdateInsertButton() {
	if e.insert == nil {
		return
	}
	if len(e.variableDefs()) > 0 {
		e.insert.Enable()
		return
	}
	e.insert.Disable()
}

func (e *VarNameEntry) openVariablePicker() {
	e.UpdateInsertButton()
	defs := e.variableDefs()
	if len(defs) == 0 {
		return
	}
	ShowVariablePicker(e, defs, e.pickVariable)
}

func (e *VarNameEntry) pickVariable(name string) {
	e.SetText(name)
}

func (e *VarNameEntry) refreshOptions() {
	defs := e.variableDefs()
	cur := strings.TrimSpace(e.Text)
	filtered := make([]models.VariableDef, 0, len(defs))
	for _, d := range defs {
		if cur == "" || strings.HasPrefix(strings.ToLower(d.Name), strings.ToLower(cur)) {
			filtered = append(filtered, d)
		}
	}
	e.shownDefs = filtered
	names := make([]string, len(filtered))
	for i, d := range filtered {
		names[i] = d.Name
	}
	e.SetOptions(names)
}

func (e *VarNameEntry) TypedRune(r rune) {
	e.CompletionEntry.TypedRune(r)
	e.refreshOptions()
	if strings.TrimSpace(e.Text) != "" && len(e.Options) > 0 {
		e.ShowCompletion()
	}
}

func (e *VarNameEntry) FocusGained() {
	e.CompletionEntry.FocusGained()
	e.UpdateInsertButton()
	e.refreshOptions()
	if strings.TrimSpace(e.Text) != "" && len(e.Options) > 0 {
		e.ShowCompletion()
	}
}

func (e *VarNameEntry) TappedSecondary(pe *fyne.PointEvent) {
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
		menuItems = append(menuItems, fyne.NewMenuItem("Pick Variable…", func() {
			e.openVariablePicker()
		}))
	}

	driver := fyne.CurrentApp().Driver()
	entryPos := driver.AbsolutePositionForObject(e)
	popUpPos := entryPos.Add(pe.Position)
	c := driver.CanvasForObject(e)
	widget.ShowPopUpMenuAtPosition(fyne.NewMenu("", menuItems...), c, popUpPos)
}

func (e *VarNameEntry) CreateRenderer() fyne.WidgetRenderer {
	base := e.Entry.CreateRenderer()
	e.ExtendBaseWidget(e)
	return &varNameEntryRendererWrap{inner: base, entry: e}
}

// EntryText returns text from VarNameEntry or related entry types.
func EntryTextFromName(w fyne.CanvasObject) string {
	switch e := w.(type) {
	case *VarNameEntry:
		return e.Text
	default:
		return EntryText(w)
	}
}

// SetEntryText sets text on supported entry types.
func SetEntryTextOnName(w fyne.CanvasObject, text string) {
	switch e := w.(type) {
	case *VarNameEntry:
		e.SetText(text)
	default:
		SetEntryText(w, text)
	}
}

var _ fyne.Focusable = (*VarNameEntry)(nil)
