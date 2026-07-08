package dialogs

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

// modalPopupDialog implements dialog.Dialog for widget.NewModalPopUp overlays.
type modalPopupDialog struct {
	pop      *widget.PopUp
	onClosed func()
}

// WrapModalPopup returns a dialog.Dialog whose Hide dismisses the popup and runs onClosed callbacks.
func WrapModalPopup(pop *widget.PopUp) dialog.Dialog {
	return &modalPopupDialog{pop: pop}
}

func (d *modalPopupDialog) Show()                 { d.pop.Show() }
func (d *modalPopupDialog) Dismiss()              { d.Hide() }
func (d *modalPopupDialog) SetDismissText(string) {}
func (d *modalPopupDialog) SetOnClosed(closed func()) {
	if closed == nil {
		return
	}
	orig := d.onClosed
	d.onClosed = func() {
		if orig != nil {
			orig()
		}
		closed()
	}
}
func (d *modalPopupDialog) Hide() {
	d.pop.Hide()
	if d.onClosed != nil {
		cb := d.onClosed
		d.onClosed = nil
		cb()
	}
}
func (d *modalPopupDialog) Refresh()           { d.pop.Refresh() }
func (d *modalPopupDialog) Resize(s fyne.Size) { d.pop.Resize(s) }
func (d *modalPopupDialog) MinSize() fyne.Size { return d.pop.MinSize() }

var _ dialog.Dialog = (*modalPopupDialog)(nil)

// AddPopupEscapeClose wraps a modal popup as a dialog and registers Escape to dismiss it.
func AddPopupEscapeClose(pop *widget.PopUp, parent fyne.Window) dialog.Dialog {
	fynetooltip.AddPopUpToolTipLayer(pop)
	d := WrapModalPopup(pop)
	AddDialogEscapeClose(d, parent)
	return d
}
