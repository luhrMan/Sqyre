package custom_widgets

import (
	"image/color"
	"strings"

	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

func textContainsVarRef(text string) bool {
	return services.TextContainsVarRef(text)
}

func NewVariableRefPill(name string, unknown bool) fyne.CanvasObject {
	actionType := "setvariable"
	if unknown {
		actionType = "warning"
	}
	return actions.NewDisplayPill(name, actionType)
}

func varRefLineHeight(textStyle fyne.TextStyle) float32 {
	th := theme.Current()
	textSize := th.Size(theme.SizeNameText)
	lineSpace := th.Size(theme.SizeNameLineSpacing)
	return fyne.MeasureText("Mg", textSize, textStyle).Height + lineSpace
}

func buildVarRefLineDisplay(line string, textStyle fyne.TextStyle, known map[string]bool) fyne.CanvasObject {
	segs := services.ParseVarRefSegments(line)
	row := container.NewHBox()
	for _, seg := range segs {
		if seg.IsRef {
			unknown := !known[strings.ToLower(strings.TrimSpace(seg.Name))]
			row.Add(NewVariableRefPill(seg.Name, unknown))
			continue
		}
		if seg.Text == "" {
			continue
		}
		txt := canvas.NewText(seg.Text, theme.Color(theme.ColorNameForeground))
		txt.TextSize = theme.TextSize()
		txt.TextStyle = textStyle
		txt.Alignment = fyne.TextAlignLeading
		row.Add(txt)
	}
	minH := varRefLineHeight(textStyle)
	spacer := canvas.NewRectangle(color.Transparent)
	spacer.SetMinSize(fyne.NewSize(0, minH))
	return container.NewStack(spacer, row)
}

func buildVarRefDisplay(text string, multiLine bool, textStyle fyne.TextStyle, known map[string]bool) fyne.CanvasObject {
	if !multiLine {
		return buildVarRefLineDisplay(text, textStyle, known)
	}
	lines := splitLines(text)
	if len(lines) == 0 {
		return container.NewHBox()
	}
	if len(lines) == 1 {
		return buildVarRefLineDisplay(lines[0], textStyle, known)
	}
	box := container.NewVBox()
	for _, line := range lines {
		box.Add(buildVarRefLineDisplay(line, textStyle, known))
	}
	return box
}

type variableRefOverlay struct {
	bg      *canvas.Rectangle
	scroll  *container.Scroll
	root    *fyne.Container
	host    *pillOverlayHost
	visible bool

	lastText   string
	lastKnown  string
	lastShow   bool
	lastMulti  bool
	cachedDisp fyne.CanvasObject
}

func newVariableRefOverlay() *variableRefOverlay {
	bg := canvas.NewRectangle(theme.Color(theme.ColorNameInputBackground))
	scroll := container.NewScroll(container.NewHBox())
	scroll.Direction = container.ScrollBoth
	root := container.NewStack(bg, scroll)
	root.Hide()
	return &variableRefOverlay{bg: bg, scroll: scroll, root: root}
}

func knownSetFingerprint(known map[string]bool) string {
	if len(known) == 0 {
		return ""
	}
	var b strings.Builder
	for k := range known {
		b.WriteString(k)
		b.WriteByte(';')
	}
	return b.String()
}

func (o *variableRefOverlay) sync(text string, multiLine bool, textStyle fyne.TextStyle, show bool, known map[string]bool) {
	show = show && text != "" && textContainsVarRef(text)
	if !show {
		o.visible = false
		o.root.Hide()
		if o.host != nil {
			o.host.Hide()
		}
		return
	}

	fp := knownSetFingerprint(known)
	if o.cachedDisp != nil && o.lastText == text && o.lastMulti == multiLine && o.lastKnown == fp && o.lastShow == show {
		o.visible = true
		o.root.Show()
		if o.host != nil {
			o.host.Show()
		}
		return
	}

	o.lastText = text
	o.lastMulti = multiLine
	o.lastKnown = fp
	o.lastShow = show

	th := theme.Current()
	inputBorder := th.Size(theme.SizeNameInputBorder)
	o.bg.FillColor = theme.Color(theme.ColorNameInputBackground)
	o.bg.Refresh()

	display := buildVarRefDisplay(text, multiLine, textStyle, known)
	o.cachedDisp = display
	topPad := canvas.NewRectangle(color.Transparent)
	topPad.SetMinSize(fyne.NewSize(0, inputBorder))
	o.scroll.Content = container.NewBorder(topPad, nil, nil, nil, display)
	o.scroll.Offset = fyne.NewPos(0, 0)
	o.scroll.Refresh()
	o.visible = true
	o.root.Show()
	if o.host != nil {
		o.host.Show()
	}
}
