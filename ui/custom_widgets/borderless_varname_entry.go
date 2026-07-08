package custom_widgets

import (
	"Sqyre/internal/models"
	"image/color"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
)

// BorderlessVarNameEntry is a single-line entry for naming macro variables that
// renders like a nested variable pill when unfocused.
type BorderlessVarNameEntry struct {
	VarNameEntry
	themeScope *container.ThemeOverride
}

// NewBorderlessVarNameEntry creates an entry without background or border styling.
func NewBorderlessVarNameEntry(getDefs func() []models.VariableDef) *BorderlessVarNameEntry {
	e := &BorderlessVarNameEntry{}
	e.GetVariableDefs = getDefs
	e.Wrapping = fyne.TextWrapOff
	e.Scroll = fyne.ScrollNone
	e.ExtendBaseWidget(e)
	e.CanvasHost = e
	e.pillOverlayBorderless = true
	e.OnChanged = e.handleChanged
	e.initRichCompletion()
	e.themeScope = container.NewThemeOverride(e, newBorderlessEntryTheme())
	return e
}

func (e *BorderlessVarNameEntry) ExtendBaseWidget(wid fyne.Widget) {
	e.Entry.ExtendBaseWidget(wid)
}

func (e *BorderlessVarNameEntry) MinSize() fyne.Size {
	e.ExtendBaseWidget(e)
	return e.textMinSize()
}

func (e *BorderlessVarNameEntry) textMinSize() fyne.Size {
	textSize := PillTextSize()
	text := e.Text
	if text == "" {
		text = " "
	}
	var width float32
	if e.hideTextForPills && strings.TrimSpace(text) != "" {
		width = BuildVariableNamePillContent(text, e.knownVariables()).MinSize().Width
	} else {
		width = fyne.MeasureText(text, textSize, e.TextStyle).Width
	}
	const cursorSlack float32 = 2
	return fyne.NewSize(width+cursorSlack, PillLineHeight())
}

func (e *BorderlessVarNameEntry) CreateRenderer() fyne.WidgetRenderer {
	e.ExtendBaseWidget(e)
	return &borderlessVarNameEntryRenderer{
		inner: e.VarNameEntry.CreateRenderer(),
		entry: e,
	}
}

type borderlessVarNameEntryRenderer struct {
	inner fyne.WidgetRenderer
	entry *BorderlessVarNameEntry
}

func (r *borderlessVarNameEntryRenderer) hideChrome() {
	for i, obj := range r.inner.Objects() {
		if i > 1 {
			break
		}
		rect, ok := obj.(*canvas.Rectangle)
		if !ok {
			continue
		}
		rect.FillColor = color.Transparent
		rect.StrokeColor = color.Transparent
		rect.StrokeWidth = 0
	}
}

func (r *borderlessVarNameEntryRenderer) Layout(size fyne.Size) {
	min := r.MinSize()
	if size.Width > min.Width {
		size.Width = min.Width
	}
	size.Height = min.Height
	r.inner.Layout(size)
	r.hideChrome()
}

func (r *borderlessVarNameEntryRenderer) MinSize() fyne.Size {
	return r.entry.textMinSize()
}

func (r *borderlessVarNameEntryRenderer) Refresh() {
	r.inner.Refresh()
	r.hideChrome()
}

func (r *borderlessVarNameEntryRenderer) Objects() []fyne.CanvasObject {
	return r.inner.Objects()
}

func (r *borderlessVarNameEntryRenderer) Destroy() {
	r.inner.Destroy()
}
