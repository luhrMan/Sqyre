package custom_widgets

import (
	"Sqyre/internal/models"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

// BorderlessEntry is a single-line entry that renders like plain text (no input chrome)
// with ${variable} completion and picker support.
type BorderlessEntry struct {
	VarEntry
	themeScope *container.ThemeOverride
}

// NewBorderlessEntry creates an entry without background or border styling.
// getDefs supplies macro variable metadata for completion; pass nil to disable.
func NewBorderlessEntry(getDefs func() []models.VariableDef) *BorderlessEntry {
	e := &BorderlessEntry{}
	e.GetVariableDefs = getDefs
	e.Wrapping = fyne.TextWrapOff
	e.Scroll = fyne.ScrollNone
	e.ExtendBaseWidget(e)
	e.CanvasHost = e
	e.pillOverlayBorderless = true
	e.initCompletion()
	e.themeScope = container.NewThemeOverride(e, newBorderlessEntryTheme())
	return e
}

func (e *BorderlessEntry) ExtendBaseWidget(wid fyne.Widget) {
	e.Entry.ExtendBaseWidget(wid)
}

// MinSize returns the space required to show the full text without scrolling.
func (e *BorderlessEntry) MinSize() fyne.Size {
	e.ExtendBaseWidget(e)
	return e.textMinSize()
}

func (e *BorderlessEntry) textMinSize() fyne.Size {
	textSize := PillTextSize()
	text := e.Text
	if text == "" {
		text = " "
	}
	var width float32
	if e.hideTextForPills && textContainsVarRef(text) {
		width = BuildVarRefPillContent(text, e.knownVariables()).MinSize().Width
	} else {
		width = fyne.MeasureText(text, textSize, e.TextStyle).Width
	}
	const cursorSlack float32 = 2
	return fyne.NewSize(width+cursorSlack, PillLineHeight())
}

func newBorderlessEntryTheme() fyne.Theme {
	base := theme.Current()
	if app := fyne.CurrentApp(); app != nil {
		base = app.Settings().Theme()
	}
	return borderlessEntryTheme{Theme: base}
}

type borderlessEntryTheme struct {
	fyne.Theme
}

func (t borderlessEntryTheme) Size(name fyne.ThemeSizeName) float32 {
	switch name {
	case theme.SizeNameText:
		return PillTextSize()
	case theme.SizeNameInnerPadding:
		return 0
	case theme.SizeNameLineSpacing:
		return 0
	case theme.SizeNameInputBorder:
		return 0
	default:
		return t.Theme.Size(name)
	}
}

func (e *BorderlessEntry) CreateRenderer() fyne.WidgetRenderer {
	e.ExtendBaseWidget(e)
	return &borderlessEntryRenderer{
		inner: e.VarEntry.CreateRenderer(),
		entry: e,
	}
}

type borderlessEntryRenderer struct {
	inner fyne.WidgetRenderer
	entry *BorderlessEntry
}

func (r *borderlessEntryRenderer) hideChrome() {
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

func (r *borderlessEntryRenderer) Layout(size fyne.Size) {
	min := r.MinSize()
	if size.Width > min.Width {
		size.Width = min.Width
	}
	size.Height = min.Height
	r.inner.Layout(size)
	r.hideChrome()
}

func (r *borderlessEntryRenderer) MinSize() fyne.Size {
	return r.entry.textMinSize()
}

func (r *borderlessEntryRenderer) Refresh() {
	r.inner.Refresh()
	r.hideChrome()
}

func (r *borderlessEntryRenderer) Objects() []fyne.CanvasObject {
	return r.inner.Objects()
}

func (r *borderlessEntryRenderer) Destroy() {
	r.inner.Destroy()
}
