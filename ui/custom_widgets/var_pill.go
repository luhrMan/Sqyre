package custom_widgets

import (
	"image/color"
	"regexp"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
)

var varRefPattern = regexp.MustCompile(`\$\{([^}]+)\}`)

type varTextSegment struct {
	text  string
	isRef bool
	name  string
}

func parseVarRefSegments(line string) []varTextSegment {
	if line == "" {
		return nil
	}
	locs := varRefPattern.FindAllStringSubmatchIndex(line, -1)
	if len(locs) == 0 {
		return []varTextSegment{{text: line}}
	}
	segs := make([]varTextSegment, 0, len(locs)*2+1)
	last := 0
	for _, loc := range locs {
		if loc[0] > last {
			segs = append(segs, varTextSegment{text: line[last:loc[0]]})
		}
		segs = append(segs, varTextSegment{
			text:  line[loc[0]:loc[1]],
			isRef: true,
			name:  line[loc[2]:loc[3]],
		})
		last = loc[1]
	}
	if last < len(line) {
		segs = append(segs, varTextSegment{text: line[last:]})
	}
	return segs
}

func textContainsVarRef(text string) bool {
	return varRefPattern.MatchString(text)
}

func NewVariableRefPill(name string) fyne.CanvasObject {
	return actions.NewDisplayPill(name, "setvariable")
}

func varRefLineHeight(textStyle fyne.TextStyle) float32 {
	th := theme.Current()
	textSize := th.Size(theme.SizeNameText)
	lineSpace := th.Size(theme.SizeNameLineSpacing)
	return fyne.MeasureText("Mg", textSize, textStyle).Height + lineSpace
}

func buildVarRefLineDisplay(line string, textStyle fyne.TextStyle) fyne.CanvasObject {
	segs := parseVarRefSegments(line)
	row := container.NewHBox()
	for _, seg := range segs {
		if seg.isRef {
			row.Add(NewVariableRefPill(seg.name))
			continue
		}
		if seg.text == "" {
			continue
		}
		txt := canvas.NewText(seg.text, theme.Color(theme.ColorNameForeground))
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

func buildVarRefDisplay(text string, multiLine bool, textStyle fyne.TextStyle) fyne.CanvasObject {
	if !multiLine {
		return buildVarRefLineDisplay(text, textStyle)
	}
	lines := splitLines(text)
	if len(lines) == 0 {
		return container.NewHBox()
	}
	if len(lines) == 1 {
		return buildVarRefLineDisplay(lines[0], textStyle)
	}
	box := container.NewVBox()
	for _, line := range lines {
		box.Add(buildVarRefLineDisplay(line, textStyle))
	}
	return box
}

type variableRefOverlay struct {
	bg      *canvas.Rectangle
	scroll  *container.Scroll
	root    *fyne.Container
	host    *pillOverlayHost
	visible bool
}

func newVariableRefOverlay() *variableRefOverlay {
	bg := canvas.NewRectangle(theme.Color(theme.ColorNameInputBackground))
	scroll := container.NewScroll(container.NewHBox())
	scroll.Direction = container.ScrollBoth
	root := container.NewStack(bg, scroll)
	root.Hide()
	return &variableRefOverlay{bg: bg, scroll: scroll, root: root}
}

func (o *variableRefOverlay) sync(text string, multiLine bool, textStyle fyne.TextStyle, show bool) {
	o.visible = show && text != "" && textContainsVarRef(text)
	if !o.visible {
		o.root.Hide()
		if o.host != nil {
			o.host.Hide()
		}
		return
	}
	th := theme.Current()
	inputBorder := th.Size(theme.SizeNameInputBorder)
	o.bg.FillColor = theme.Color(theme.ColorNameInputBackground)
	o.bg.Refresh()

	display := buildVarRefDisplay(text, multiLine, textStyle)
	topPad := canvas.NewRectangle(color.Transparent)
	topPad.SetMinSize(fyne.NewSize(0, inputBorder))
	o.scroll.Content = container.NewBorder(topPad, nil, nil, nil, display)
	o.scroll.Offset = fyne.NewPos(0, 0)
	o.scroll.Refresh()
	o.root.Show()
	if o.host != nil {
		o.host.Show()
	}
}
