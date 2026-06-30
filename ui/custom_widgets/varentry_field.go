package custom_widgets

import (
	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"sync"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

const validationDebounce = 150 * time.Millisecond

// VarEntryField wraps a VarEntry with validation feedback on a trailing icon tooltip.
// Warnings (e.g. unknown variables) are shown but do not block submission.
// Errors (e.g. invalid expressions) are shown and block submission.
type VarEntryField struct {
	widget.BaseWidget

	Entry        *VarEntry
	feedbackIcon *ttwidget.Icon
	previewLabel *widget.Label

	Validate func(text string) services.EntryValidation
	last     services.EntryValidation
	lastMsg  string

	// ResolvePreview, when set, formats a resolved-value hint shown below the field when unfocused.
	ResolvePreview func(text string) string

	OnChanged           func(string)
	onValidationChanged func()

	validateMu     sync.Mutex
	validateTimer  *time.Timer
	pendingText    string
}

// NewVarEntryField creates a single-line variable entry with validation.
func NewVarEntryField(getVars func() []string, validate func(text string) services.EntryValidation) *VarEntryField {
	return newVarEntryFieldWithEntry(NewVarEntry(getVars), validate)
}

// NewVarEntryFieldWithDefs creates a validated field backed by variable definitions.
func NewVarEntryFieldWithDefs(getDefs func() []models.VariableDef, validate func(text string) services.EntryValidation) *VarEntryField {
	return newVarEntryFieldWithEntry(NewVarEntryWithDefs(getDefs), validate)
}

// NewMultiLineVarEntryField creates a multi-line variable entry with validation.
func NewMultiLineVarEntryField(getVars func() []string, validate func(text string) services.EntryValidation) *VarEntryField {
	return newVarEntryFieldWithEntry(NewMultiLineVarEntry(getVars), validate)
}

// NewMultiLineVarEntryFieldWithDefs creates a validated multi-line field backed by definitions.
func NewMultiLineVarEntryFieldWithDefs(getDefs func() []models.VariableDef, validate func(text string) services.EntryValidation) *VarEntryField {
	return newVarEntryFieldWithEntry(NewMultiLineVarEntryWithDefs(getDefs), validate)
}

func newVarEntryFieldWithEntry(entry *VarEntry, validate func(text string) services.EntryValidation) *VarEntryField {
	f := &VarEntryField{
		Entry:        entry,
		feedbackIcon: newValidationFeedbackIcon(),
		previewLabel: widget.NewLabel(""),
		Validate:     validate,
	}
	f.previewLabel.Wrapping = fyne.TextWrapWord
	f.previewLabel.Hide()
	f.Entry.SetFeedbackIcon(f.feedbackIcon)
	f.Entry.ChangedFn = f.scheduleValidation
	f.Entry.FocusChangedFn = func(focused bool) {
		if focused {
			f.previewLabel.Hide()
			f.Refresh()
			return
		}
		f.syncPreview(f.Entry.Text)
		f.Refresh()
	}
	f.ExtendBaseWidget(f)
	f.applyValidation(f.Entry.Text)
	return f
}

func newValidationFeedbackIcon() *ttwidget.Icon {
	icon := ttwidget.NewIcon(theme.WarningIcon())
	icon.Hide()
	return icon
}

func (f *VarEntryField) scheduleValidation(text string) {
	f.validateMu.Lock()
	defer f.validateMu.Unlock()
	f.pendingText = text
	if f.validateTimer != nil {
		f.validateTimer.Stop()
	}
	f.validateTimer = time.AfterFunc(validationDebounce, func() {
		f.validateMu.Lock()
		t := f.pendingText
		f.validateMu.Unlock()
		f.applyValidation(t)
	})
}

func (f *VarEntryField) applyValidation(text string) {
	prevBlocks := f.last.BlocksSubmit()
	prevMsg := f.lastMsg
	if f.Validate == nil {
		f.last = services.EntryValidation{}
	} else {
		f.last = f.Validate(text)
	}

	msg, importance := f.feedbackMessage()
	f.lastMsg = msg
	if msg == "" {
		f.feedbackIcon.Hide()
		f.feedbackIcon.SetToolTip("")
	} else {
		if importance == widget.DangerImportance {
			f.feedbackIcon.SetResource(theme.ErrorIcon())
		} else {
			f.feedbackIcon.SetResource(theme.WarningIcon())
		}
		f.feedbackIcon.SetToolTip(msg)
		f.feedbackIcon.Show()
	}

	f.syncPreview(text)

	newBlocks := f.last.BlocksSubmit()
	if prevBlocks != newBlocks || prevMsg != msg {
		f.feedbackIcon.Refresh()
		f.Entry.Refresh()
		if f.onValidationChanged != nil {
			f.onValidationChanged()
		}
	}
	f.Refresh()
	if f.OnChanged != nil {
		f.OnChanged(text)
	}
}

func (f *VarEntryField) syncPreview(text string) {
	if f.ResolvePreview == nil || f.Entry.hasFocus || f.last.Error != "" {
		f.previewLabel.Hide()
		return
	}
	hint := f.ResolvePreview(text)
	if hint == "" {
		f.previewLabel.Hide()
		return
	}
	f.previewLabel.SetText(hint)
	f.previewLabel.Show()
}

func (f *VarEntryField) feedbackMessage() (string, widget.Importance) {
	if f.last.Error != "" {
		return f.last.Error, widget.DangerImportance
	}
	if f.last.Warning != "" {
		return f.last.Warning, widget.WarningImportance
	}
	return "", widget.MediumImportance
}

// Valid reports whether the current text may be submitted.
func (f *VarEntryField) Valid() bool {
	return !f.last.BlocksSubmit()
}

// ValidationWarning returns the current non-blocking warning, if any.
func (f *VarEntryField) ValidationWarning() string {
	return f.last.Warning
}

// ValidationError returns the current blocking error, if any.
func (f *VarEntryField) ValidationError() string {
	return f.last.Error
}

// SetOnValidationChanged registers a callback invoked when blocking validity changes.
func (f *VarEntryField) SetOnValidationChanged(fn func()) {
	f.onValidationChanged = fn
}

// Revalidate runs validation against the current entry text immediately.
func (f *VarEntryField) Revalidate() {
	f.validateMu.Lock()
	if f.validateTimer != nil {
		f.validateTimer.Stop()
	}
	f.validateMu.Unlock()
	f.applyValidation(f.Entry.Text)
}

func (f *VarEntryField) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(container.NewVBox(f.Entry, f.previewLabel))
}
