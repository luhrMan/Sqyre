package actiondisplay

import (
	"context"
	"image/color"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

const pillTooltipShowDelay = 500 * time.Millisecond

// TooltipSink shows hover tips above action-tooltip edit content (fyne-tooltip layers sit below).
type TooltipSink interface {
	ShowTooltip(text string, absPos fyne.Position)
	HideTooltip()
}

type pillStepperTipTarget interface {
	scheduleTooltip(text string, absPos fyne.Position)
	cancelTooltip()
}

// pillTipHover is a transparent hover target for stepper value min/max tooltips.
type pillTipHover struct {
	fynewidget.BaseWidget

	owner   pillStepperTipTarget
	tipText string
}

func newPillTipHover(owner pillStepperTipTarget) *pillTipHover {
	h := &pillTipHover{owner: owner}
	h.ExtendBaseWidget(h)
	return h
}

func (h *pillTipHover) SetToolTip(text string) {
	h.tipText = text
}

func (h *pillTipHover) MouseIn(e *desktop.MouseEvent) {
	if h.tipText == "" || h.owner == nil {
		return
	}
	h.owner.scheduleTooltip(h.tipText, e.AbsolutePosition)
}

func (h *pillTipHover) MouseOut() {
	if h.owner != nil {
		h.owner.cancelTooltip()
	}
}

func (h *pillTipHover) MouseMoved(*desktop.MouseEvent) {}

func (h *pillTipHover) CreateRenderer() fyne.WidgetRenderer {
	return fynewidget.NewSimpleRenderer(canvas.NewRectangle(color.Transparent))
}

type pillTipButton struct {
	fynewidget.Button

	owner   pillStepperTipTarget
	tipText string
}

func newPillTipButton(icon fyne.Resource, tipText string, owner pillStepperTipTarget, onTapped func()) *pillTipButton {
	b := &pillTipButton{owner: owner, tipText: tipText}
	b.Text = ""
	b.Icon = icon
	b.OnTapped = onTapped
	b.ExtendBaseWidget(b)
	return b
}

func (b *pillTipButton) SetToolTip(text string) {
	b.tipText = text
}

func (b *pillTipButton) MouseIn(e *desktop.MouseEvent) {
	b.Button.MouseIn(e)
	if b.tipText != "" && b.owner != nil {
		b.owner.scheduleTooltip(b.tipText, e.AbsolutePosition)
	}
}

func (b *pillTipButton) MouseOut() {
	if b.owner != nil {
		b.owner.cancelTooltip()
	}
	b.Button.MouseOut()
}

func (b *pillTipButton) MouseMoved(e *desktop.MouseEvent) {
	b.Button.MouseMoved(e)
}

type pillStepperTooltipState struct {
	sink          TooltipSink
	pendingCancel context.CancelFunc
}

func (s *pillStepperTooltipState) bindSink(sink TooltipSink) {
	s.sink = sink
}

func (s *pillStepperTooltipState) scheduleTooltip(text string, absPos fyne.Position) {
	s.cancelTooltip()
	if text == "" || s.sink == nil {
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	s.pendingCancel = cancel
	go func() {
		select {
		case <-time.After(pillTooltipShowDelay):
		case <-ctx.Done():
			return
		}
		select {
		case <-ctx.Done():
			return
		default:
			fyne.Do(func() {
				if s.pendingCancel == nil || s.sink == nil {
					return
				}
				s.sink.ShowTooltip(text, absPos)
			})
		}
	}()
}

func (s *pillStepperTooltipState) cancelTooltip() {
	if s.pendingCancel != nil {
		s.pendingCancel()
		s.pendingCancel = nil
	}
	if s.sink != nil {
		s.sink.HideTooltip()
	}
}

// BindPillStepperTooltips wires compact pill steppers under root to sink.
func BindPillStepperTooltips(root fyne.CanvasObject, sink TooltipSink) {
	if root == nil || sink == nil {
		return
	}
	switch w := root.(type) {
	case *PillIntStepper:
		w.BindTooltipSink(sink)
	case *PillFloatStepper:
		w.BindTooltipSink(sink)
	}
	if cont, ok := root.(*fyne.Container); ok {
		for _, obj := range cont.Objects {
			BindPillStepperTooltips(obj, sink)
		}
	}
}

const pillHoverTipBelowMouse = 14

// NewPillHoverTipPanel renders a single-line hover tip for the action-tooltip panel layer.
func NewPillHoverTipPanel(text string) fyne.CanvasObject {
	v := fyne.CurrentApp().Settings().ThemeVariant()
	th := fyne.CurrentApp().Settings().Theme()
	bg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	bg.CornerRadius = 4
	lbl := canvas.NewText(text, th.Color(theme.ColorNameForeground, v))
	lbl.TextSize = th.Size(theme.SizeNameCaptionText)
	innerPad := th.Size(theme.SizeNameInnerPadding)
	textSize := fyne.MeasureText(text, lbl.TextSize, fyne.TextStyle{})
	size := fyne.NewSize(textSize.Width+innerPad*2, textSize.Height+innerPad)
	bg.Resize(size)
	lbl.Move(fyne.NewPos(innerPad, innerPad/2))
	return container.NewStack(bg, lbl)
}

// PositionPillHoverTip places tip near absPos relative to panelOrigin.
func PositionPillHoverTip(tip fyne.CanvasObject, panelOrigin, absPos fyne.Position) {
	rel := absPos.Subtract(panelOrigin)
	rel.Y += pillHoverTipBelowMouse
	tip.Move(rel)
}
