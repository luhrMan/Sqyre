package custom_widgets

import (
	"image/color"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

// BuildVariableNamePillContent renders a variable name as a compact nested pill chip.
func BuildVariableNamePillContent(name string, known map[string]bool) fyne.CanvasObject {
	name = strings.TrimSpace(name)
	if name == "" {
		return container.NewHBox()
	}
	unknown := !known[strings.ToLower(name)]
	return NewNestedVariableRefPill(name, unknown)
}

type variableNameOverlay struct {
	bg      *canvas.Rectangle
	scroll  *container.Scroll
	root    *fyne.Container
	host    *varNamePillOverlayHost
	visible bool

	lastText       string
	lastKnown      string
	lastShow       bool
	lastBorderless bool
	cachedDisp     fyne.CanvasObject
}

func newVariableNameOverlay() *variableNameOverlay {
	bg := canvas.NewRectangle(theme.Color(theme.ColorNameInputBackground))
	scroll := container.NewScroll(container.NewHBox())
	scroll.Direction = container.ScrollBoth
	root := container.NewStack(bg, scroll)
	root.Hide()
	return &variableNameOverlay{bg: bg, scroll: scroll, root: root}
}

func (o *variableNameOverlay) sync(text string, show bool, known map[string]bool, borderless bool) {
	text = strings.TrimSpace(text)
	show = show && text != ""
	if !show {
		o.visible = false
		o.root.Hide()
		if o.host != nil {
			o.host.Hide()
		}
		return
	}

	fp := knownSetFingerprint(known)
	if o.cachedDisp != nil && o.lastText == text && o.lastKnown == fp && o.lastShow == show && o.lastBorderless == borderless {
		o.visible = true
		o.root.Show()
		if o.host != nil {
			o.host.Show()
		}
		return
	}

	o.lastText = text
	o.lastKnown = fp
	o.lastShow = show
	o.lastBorderless = borderless

	if borderless {
		o.bg.FillColor = color.Transparent
	} else {
		o.bg.FillColor = theme.Color(theme.ColorNameInputBackground)
	}
	o.bg.Refresh()

	display := BuildVariableNamePillContent(text, known)
	o.cachedDisp = display
	o.scroll.Direction = container.ScrollNone
	o.scroll.Content = display
	o.scroll.Offset = fyne.NewPos(0, 0)
	o.scroll.Refresh()
	o.visible = true
	o.root.Show()
	if o.host != nil {
		o.host.Show()
	}
}

func (o *variableNameOverlay) object(entry *VarNameEntry) fyne.CanvasObject {
	if o.host == nil {
		o.host = newVarNamePillOverlayHost(o.root, entry)
	}
	return o.host
}
