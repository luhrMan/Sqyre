package custom_widgets

import (
	"Sqyre/ui/completionentry"
	"strings"

	"fyne.io/fyne/v2"
)

// VarNameEntry is for fields that define a variable name (not ${references}).
// It offers autocomplete from existing macro variable names.
type VarNameEntry struct {
	completionentry.CompletionEntry
	getNames func() []string
}

// NewVarNameEntry creates an entry for naming macro variables.
func NewVarNameEntry(getNames func() []string) *VarNameEntry {
	e := &VarNameEntry{getNames: getNames}
	e.ExtendBaseWidget(e)
	e.OnChanged = func(_ string) {
		e.refreshOptions()
	}
	return e
}

func (e *VarNameEntry) refreshOptions() {
	if e.getNames == nil {
		return
	}
	names := e.getNames()
	cur := strings.TrimSpace(e.Text)
	filtered := make([]string, 0, len(names))
	for _, n := range names {
		if cur == "" || strings.HasPrefix(strings.ToLower(n), strings.ToLower(cur)) {
			filtered = append(filtered, n)
		}
	}
	e.SetOptions(filtered)
}

// TypedRune shows completion after typing.
func (e *VarNameEntry) TypedRune(r rune) {
	e.CompletionEntry.TypedRune(r)
	e.refreshOptions()
	if len(e.Options) > 0 {
		e.ShowCompletion()
	}
}

// FocusGained refreshes completion options when focused.
func (e *VarNameEntry) FocusGained() {
	e.CompletionEntry.FocusGained()
	e.refreshOptions()
}

// TappedSecondary shows standard entry menu without variable-reference insertion.
func (e *VarNameEntry) TappedSecondary(pe *fyne.PointEvent) {
	e.Entry.TappedSecondary(pe)
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

// Ensure VarNameEntry satisfies widget focus interface via embedded Entry.
var _ fyne.Focusable = (*VarNameEntry)(nil)
