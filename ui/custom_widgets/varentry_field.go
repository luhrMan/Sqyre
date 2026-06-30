package custom_widgets

import (
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// VarEntryField wraps a VarEntry with validation feedback on a trailing icon tooltip.
// Warnings (e.g. unknown variables) are shown but do not block submission.
// Errors (e.g. invalid expressions) are shown and block submission.
type VarEntryField struct {
	widget.BaseWidget

	Entry        *VarEntry
	feedbackIcon *ttwidget.Icon

	Validate func(text string) services.EntryValidation
	last     services.EntryValidation
	lastMsg  string

	OnChanged           func(string)
	onValidationChanged func()
}

// NewVarEntryField creates a single-line variable entry with validation.
func NewVarEntryField(getVars func() []string, validate func(text string) services.EntryValidation) *VarEntryField {
	f := &VarEntryField{
		Entry:        NewVarEntry(getVars),
		feedbackIcon: newValidationFeedbackIcon(),
		Validate:     validate,
	}
	f.Entry.SetFeedbackIcon(f.feedbackIcon)
	f.Entry.ChangedFn = f.refreshValidation
	f.ExtendBaseWidget(f)
	f.refreshValidation(f.Entry.Text)
	return f
}

// NewMultiLineVarEntryField creates a multi-line variable entry with validation.
func NewMultiLineVarEntryField(getVars func() []string, validate func(text string) services.EntryValidation) *VarEntryField {
	f := &VarEntryField{
		Entry:        NewMultiLineVarEntry(getVars),
		feedbackIcon: newValidationFeedbackIcon(),
		Validate:     validate,
	}
	f.Entry.SetFeedbackIcon(f.feedbackIcon)
	f.Entry.ChangedFn = f.refreshValidation
	f.ExtendBaseWidget(f)
	f.refreshValidation(f.Entry.Text)
	return f
}

func newValidationFeedbackIcon() *ttwidget.Icon {
	icon := ttwidget.NewIcon(theme.WarningIcon())
	icon.Hide()
	return icon
}

func (f *VarEntryField) refreshValidation(text string) {
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

	newBlocks := f.last.BlocksSubmit()
	if prevBlocks != newBlocks || prevMsg != msg {
		f.feedbackIcon.Refresh()
		f.Entry.Refresh()
		if f.onValidationChanged != nil {
			f.onValidationChanged()
		}
	}
	if f.OnChanged != nil {
		f.OnChanged(text)
	}
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

// Revalidate runs validation against the current entry text.
func (f *VarEntryField) Revalidate() {
	f.refreshValidation(f.Entry.Text)
}

func (f *VarEntryField) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(f.Entry)
}
