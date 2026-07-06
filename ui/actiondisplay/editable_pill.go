package actiondisplay

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// NewPillInlineField lays out a caption-sized label beside a compact field on one line.
func NewPillInlineField(label string, field fyne.CanvasObject) fyne.CanvasObject {
	return newPillInlineRow(NewPillText(label), field)
}

// NewEditablePill wraps a caption label and field in the same chip style as display pills.
func NewEditablePill(label string, field fyne.CanvasObject, actionType string) fyne.CanvasObject {
	return PillChrome(NewPillInlineField(label+": ", field), actionType)
}

func newPillInlineRow(label, field fyne.CanvasObject) fyne.CanvasObject {
	r := &pillInlineRow{label: label, field: field}
	r.ExtendBaseWidget(r)
	return r
}

type pillInlineRow struct {
	widget.BaseWidget
	label fyne.CanvasObject
	field fyne.CanvasObject
}

func (r *pillInlineRow) CreateRenderer() fyne.WidgetRenderer {
	return &pillInlineRowRenderer{row: r}
}

type pillInlineRowRenderer struct {
	row *pillInlineRow
}

func (r *pillInlineRowRenderer) Layout(size fyne.Size) {
	rowH := PillLineHeight()

	labelSize := r.row.label.MinSize()
	r.row.label.Resize(fyne.NewSize(labelSize.Width, rowH))
	r.row.label.Move(fyne.NewPos(0, 0))

	fieldMin := r.row.field.MinSize()
	fieldW := fieldMin.Width
	if fieldW > size.Width-labelSize.Width {
		fieldW = size.Width - labelSize.Width
	}
	if fieldW < fieldMin.Width {
		fieldW = fieldMin.Width
	}
	r.row.field.Resize(fyne.NewSize(fieldW, rowH))
	r.row.field.Move(fyne.NewPos(labelSize.Width, 0))
}

func (r *pillInlineRowRenderer) MinSize() fyne.Size {
	labelW := r.row.label.MinSize().Width
	fieldW := r.row.field.MinSize().Width
	return fyne.NewSize(labelW+fieldW, PillLineHeight())
}

func (r *pillInlineRowRenderer) Refresh() {
	r.row.label.Refresh()
	r.row.field.Refresh()
}

func (r *pillInlineRowRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.row.label, r.row.field}
}

func (r *pillInlineRowRenderer) Destroy() {}

// WrapPillStepper wraps a pill-sized stepper in pill chrome.
func WrapPillStepper(stepper fyne.CanvasObject, actionType string) fyne.CanvasObject {
	return PillChrome(stepper, actionType)
}
