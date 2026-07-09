package macro

import (
	"image"

	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/desktopview"
	"Sqyre/ui/dialogs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	fynewidget "fyne.io/fyne/v2/widget"
)

type actionDisplayTooltipPanel struct {
	fynewidget.BaseWidget

	owner       *actionDisplayTooltipHover
	withPreview bool
	actionType  string
	extra       []actions.Param

	editing  bool
	editForm *tooltipEditForm

	enterSaveUnregister func()

	viewParamPills    fyne.CanvasObject
	viewParamPillsKey string

	viewParamPillsBodyIndex int
	viewBodyBuilt           bool

	img            *canvas.Image
	previewViewer  *custom_widgets.ZoomableImageView
	previewImage   image.Image
	message        *fynewidget.Label
	caption   *fynewidget.Label
	loading   bool
	showImage bool

	body *fyne.Container

	hoverTipLayer  *fyne.Container
	activeHoverTip fyne.CanvasObject

	// layoutSize caches tooltip dimensions; row-wrapped target icons make
	// preferredContentWidth/contentSize O(n) and run on every mouse move without this.
	layoutSize        fyne.Size
	layoutCanvasWidth float32
	layoutSizeOK      bool

	targetIconsSection fyne.CanvasObject
	targetIconsKey     string
}

func newActionDisplayTooltipPanel(owner *actionDisplayTooltipHover) *actionDisplayTooltipPanel {
	p := &actionDisplayTooltipPanel{
		owner:       owner,
		withPreview: owner.previewLoader != nil,
		actionType:  owner.actionType,
	}
	if len(owner.extra) > 0 {
		p.extra = append([]actions.Param(nil), owner.extra...)
	}
	p.ensureViewParamPills(owner)
	p.body = container.NewVBox()
	p.hoverTipLayer = container.NewWithoutLayout()
	p.rebuildBody()
	p.ExtendBaseWidget(p)
	return p
}

func (p *actionDisplayTooltipPanel) ShowTooltip(text string, absPos fyne.Position) {
	p.HideTooltip()
	if text == "" {
		return
	}
	tip := actiondisplay.NewPillHoverTipPanel(text)
	actiondisplay.PositionPillHoverTip(tip, fyne.CurrentApp().Driver().AbsolutePositionForObject(p), absPos)
	p.hoverTipLayer.Add(tip)
	p.activeHoverTip = tip
	p.hoverTipLayer.Refresh()
}

func (p *actionDisplayTooltipPanel) HideTooltip() {
	if p.hoverTipLayer == nil {
		return
	}
	p.hoverTipLayer.Objects = nil
	p.hoverTipLayer.Refresh()
	p.activeHoverTip = nil
}

func (p *actionDisplayTooltipPanel) ensureViewParamPills(owner *actionDisplayTooltipHover) fyne.CanvasObject {
	if owner == nil {
		return nil
	}
	key := viewParamPillsContentKey(owner.node)
	if p.viewParamPills != nil && p.viewParamPillsKey == key {
		return p.viewParamPills
	}
	p.viewParamPillsKey = key
	p.viewParamPills = viewParamPills(owner.node, owner.actionType)
	return p.viewParamPills
}

func (p *actionDisplayTooltipPanel) refreshViewContent(owner *actionDisplayTooltipHover) {
	prev := p.viewParamPills
	p.ensureViewParamPills(owner)
	if !p.editing && p.viewBodyBuilt && p.viewParamPillsBodyIndex >= 0 {
		if p.viewParamPillsBodyIndex < len(p.body.Objects) {
			p.body.Objects[p.viewParamPillsBodyIndex] = p.viewParamPills
		}
		// New pills can differ in size (e.g. adding/removing For-Each source rows);
		// recompute the panel geometry so the background tracks the content.
		if p.viewParamPills != prev {
			p.invalidateLayoutSize()
			p.body.Refresh()
			if owner != nil {
				owner.relayoutTooltip()
			}
		}
		return
	}
	if !p.editing {
		p.rebuildBody()
		p.Refresh()
	}
}

func (p *actionDisplayTooltipPanel) ensureTargetIconsSection(owner *actionDisplayTooltipHover) fyne.CanvasObject {
	if owner == nil {
		return nil
	}
	targets := imageSearchTargetsFromNode(owner.node)
	if len(targets) == 0 {
		p.targetIconsSection = nil
		p.targetIconsKey = ""
		return nil
	}
	key := imageSearchTargetIconsViewKey(targets)
	if p.targetIconsSection != nil && p.targetIconsKey == key {
		return p.targetIconsSection
	}
	p.targetIconsKey = key
	p.targetIconsSection = imageSearchTargetIconsView(targets)
	return p.targetIconsSection
}

func (p *actionDisplayTooltipPanel) enterEditMode() {
	if p.editing || p.owner == nil {
		return
	}
	p.owner.cancelPending()
	p.editing = true
	claimActionEditTooltip(p.owner)
	p.owner.selectRow()
	p.editForm = buildTooltipEditForm(p.owner.node, p.actionType, p.owner)
	if actionHasCoordinatePicker(p.owner.node) {
		p.withPreview = true
	}
	p.activateEnterSave()
	p.rebuildBody()
	if p.withPreview && p.owner.previewLoader == nil {
		p.setPreviewEmpty()
	}
	p.Refresh()
	p.owner.relayoutTooltip()
}

func (p *actionDisplayTooltipPanel) exitEditMode() {
	if !p.editing {
		return
	}
	p.HideTooltip()
	p.deactivateEnterSave()
	p.editing = false
	if p.owner != nil {
		releaseActionEditTooltip(p.owner)
		if p.owner.previewLoader != nil {
			p.withPreview = true
		} else {
			p.withPreview = false
		}
	}
	p.editForm = nil
	p.refreshViewContent(p.owner)
	p.rebuildBody()
	p.Refresh()
}

func (p *actionDisplayTooltipPanel) activateEnterSave() {
	p.deactivateEnterSave()
	if activeWire.RegisterTooltipEnterSave == nil {
		return
	}
	panel := p
	p.enterSaveUnregister = activeWire.RegisterTooltipEnterSave(func() {
		panel.submitEdit()
	})
}

func (p *actionDisplayTooltipPanel) deactivateEnterSave() {
	if p.enterSaveUnregister != nil {
		p.enterSaveUnregister()
		p.enterSaveUnregister = nil
	}
}

func (p *actionDisplayTooltipPanel) submitEdit() {
	if !p.editing || p.editForm == nil || p.owner == nil {
		return
	}
	if err := p.editForm.saveAction(p.owner); err != nil {
		if activeWire.Window != nil {
			dialogs.ShowErrorWithEscape(err, activeWire.Window)
		}
		return
	}
	p.owner.exitEditMode()
}

func (p *actionDisplayTooltipPanel) invalidateLayoutSize() {
	p.layoutSizeOK = false
}

// syncBackgroundLayout recomputes tooltip size/position so the background rectangle
// tracks preview and content changes (e.g. loading → image, caption updates).
func (p *actionDisplayTooltipPanel) syncBackgroundLayout() {
	p.invalidateLayoutSize()
	if p.owner == nil {
		p.Refresh()
		return
	}
	p.owner.relayoutTooltip()
	p.Refresh()
	// Preview/caption visibility changes can alter descendant MinSize only after
	// the first layout pass; reposition once more so the background tracks.
	p.invalidateLayoutSize()
	p.owner.repositionTooltip()
	p.Refresh()
}

func (p *actionDisplayTooltipPanel) measureLayoutSize(c fyne.Canvas) fyne.Size {
	canvasWidth := c.Size().Width
	if p.layoutSizeOK && p.layoutCanvasWidth == canvasWidth {
		return p.layoutSize
	}
	canvasSize := c.Size()
	edgeMarginX := canvasSize.Width * custom_widgets.TooltipEdgeMarginFraction
	maxW := canvasSize.Width - edgeMarginX*2

	natural := p.MinSize()
	width := natural.Width
	if preferred := p.preferredContentWidth(); preferred > width {
		width = preferred
	}
	if width > maxW {
		width = maxW
	}
	p.layoutSize = p.contentSize(width)
	p.layoutCanvasWidth = canvasWidth
	p.layoutSizeOK = true
	return p.layoutSize
}

func (p *actionDisplayTooltipPanel) rebuildBody() {
	p.invalidateLayoutSize()
	p.viewBodyBuilt = false
	p.viewParamPillsBodyIndex = -1
	p.body.Objects = nil
	if p.editing && p.editForm != nil {
		if p.editForm.toolbar != nil {
			p.body.Add(p.editForm.toolbar)
		}
	} else if header := actionTooltipTypeHeader(p.actionType); header != nil {
		p.body.Add(header)
	}
	if p.withPreview {
		if p.message == nil {
			p.message = fynewidget.NewLabel("Loading preview…")
			p.message.Wrapping = fyne.TextWrapWord
			p.message.Alignment = fyne.TextAlignCenter
		}
		if p.caption == nil {
			p.caption = fynewidget.NewLabel("")
			p.caption.Alignment = fyne.TextAlignCenter
			p.caption.Hide()
		}
		imageStackLayers := []fyne.CanvasObject{
			container.NewMax(p.previewDisplay()),
			container.NewPadded(p.message),
		}
		if refreshOverlay := buildPreviewRefreshOverlay(p.owner); refreshOverlay != nil {
			imageStackLayers = append(imageStackLayers, refreshOverlay)
		}
		imageStack := container.NewStack(imageStackLayers...)
		previewSection := container.NewVBox(imageStack)
		if p.caption != nil {
			previewSection.Add(p.caption)
		}
		if p.editing && p.editForm != nil && p.editForm.coordEditActions != nil {
			previewSection.Add(p.editForm.coordEditActions)
		}
		p.body.Add(wrapTooltipSection(previewSection))
	}
	if p.owner != nil {
		if p.editing && p.editForm != nil && p.editForm.targetItems != nil {
			p.body.Add(p.editForm.targetItems)
		} else if section := p.ensureTargetIconsSection(p.owner); section != nil {
			p.body.Add(section)
		}
	}
	if p.editing && p.editForm != nil {
		if p.editForm.paramPills != nil {
			p.body.Add(p.editForm.paramPills)
		}
	} else {
		if p.viewParamPills != nil {
			p.viewParamPillsBodyIndex = len(p.body.Objects)
			p.body.Add(p.viewParamPills)
		}
	}
	if p.editing && p.editForm != nil {
		actiondisplay.BindPillStepperTooltips(p.body, p)
	} else {
		p.HideTooltip()
		p.viewBodyBuilt = true
	}
	p.body.Refresh()
}

func (p *actionDisplayTooltipPanel) previewSize() fyne.Size {
	return fyne.NewSize(config.ImagePreviewMinWidth, config.ImagePreviewMinHeight)
}

func (p *actionDisplayTooltipPanel) ensureViewPreview() *canvas.Image {
	if p.img == nil {
		p.img = canvas.NewImageFromImage(nil)
		p.img.FillMode = desktopview.PreviewSnapshotFill
		p.img.SetMinSize(p.previewSize())
		if p.previewImage != nil {
			p.img.Image = p.previewImage
		}
	}
	return p.img
}

func (p *actionDisplayTooltipPanel) ensureEditPreview() *custom_widgets.ZoomableImageView {
	if p.previewViewer == nil {
		p.previewViewer = custom_widgets.NewZoomableImageView()
		p.previewViewer.SetMinSize(p.previewSize())
		if p.previewImage != nil {
			p.previewViewer.SetImage(p.previewImage)
		}
	}
	return p.previewViewer
}

// previewDisplay returns the image widget for the current mode (static in view, zoomable in edit).
func (p *actionDisplayTooltipPanel) previewDisplay() fyne.CanvasObject {
	if p.editing {
		return p.ensureEditPreview()
	}
	return p.ensureViewPreview()
}

func (p *actionDisplayTooltipPanel) applyPreviewImage() {
	if p.img != nil {
		p.img.Image = p.previewImage
	}
	if p.previewViewer != nil {
		p.previewViewer.SetImage(p.previewImage)
	}
}

func (p *actionDisplayTooltipPanel) MinSize() fyne.Size {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	size := p.body.MinSize()
	if p.withPreview {
		previewMin := p.previewSize()
		if size.Width < previewMin.Width {
			size.Width = previewMin.Width
		}
		if p.showImage {
			captionHeight := float32(0)
			if p.caption != nil && p.caption.Text != "" {
				captionHeight = p.caption.MinSize().Height
			}
			if p.editing && p.editForm != nil && p.editForm.coordEditActions != nil {
				captionHeight += p.editForm.coordEditActions.MinSize().Height
			}
			if captionHeight > 0 && previewMin.Height > size.Height {
				size.Height = previewMin.Height
			}
		}
	}
	return size.Add(fyne.NewSquareSize(innerPad * 2))
}

func (p *actionDisplayTooltipPanel) preferredContentWidth() float32 {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	w := maxRowWrapSingleLineWidth(p.body) + tooltipSectionChromeWidth()
	if p.withPreview {
		if previewMin := p.previewSize().Width; previewMin > w {
			w = previewMin
		}
	}
	return w + innerPad*2
}

func (p *actionDisplayTooltipPanel) contentSize(width float32) fyne.Size {
	innerPad := p.Theme().Size(theme.SizeNameInnerPadding)
	innerW := width - innerPad*2
	if innerW < 1 {
		innerW = 1
	}
	contentH := tooltipBodyHeightAtWidth(p.body, innerW)
	if p.withPreview && p.showImage {
		previewMin := p.previewSize()
		if contentH < previewMin.Height {
			contentH = previewMin.Height
		}
	}
	return fyne.NewSize(width, contentH+innerPad*2)
}

func (p *actionDisplayTooltipPanel) clearPreview() {
	p.loading = false
	p.showImage = false
	p.previewImage = nil
	p.applyPreviewImage()
	if p.message != nil {
		p.message.SetText("")
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
}

func (p *actionDisplayTooltipPanel) setPreviewLoading() {
	p.loading = true
	p.showImage = false
	p.previewImage = nil
	p.applyPreviewImage()
	if p.message != nil {
		p.message.SetText("Loading preview…")
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
	p.syncBackgroundLayout()
}

func (p *actionDisplayTooltipPanel) setPreviewEmpty() {
	p.loading = false
	p.showImage = false
	p.previewImage = nil
	p.applyPreviewImage()
	if p.message != nil {
		p.message.SetText("")
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
}

func (p *actionDisplayTooltipPanel) setPreviewError(msg string) {
	p.loading = false
	p.showImage = false
	if p.message != nil {
		p.message.SetText(msg)
	}
	if p.caption != nil {
		p.caption.SetText("")
		p.caption.Hide()
	}
	p.syncBackgroundLayout()
}

func (p *actionDisplayTooltipPanel) setPreviewImage(img image.Image, caption string) {
	p.loading = false
	p.showImage = true
	p.previewImage = img
	p.applyPreviewImage()
	if p.caption != nil {
		p.caption.SetText(caption)
		if caption == "" {
			p.caption.Hide()
		} else {
			p.caption.Show()
		}
	}
	p.syncBackgroundLayout()
}

func (p *actionDisplayTooltipPanel) CreateRenderer() fyne.WidgetRenderer {
	v := fyne.CurrentApp().Settings().ThemeVariant()
	th := p.Theme()
	bg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	bg.CornerRadius = 4
	bodyStack := container.NewStack(p.body, p.hoverTipLayer)
	return &actionDisplayTooltipPanelRenderer{
		panel: p,
		bg:    bg,
		body:  bodyStack,
	}
}

type actionDisplayTooltipPanelRenderer struct {
	panel *actionDisplayTooltipPanel
	bg    *canvas.Rectangle
	body  *fyne.Container
}

func (r *actionDisplayTooltipPanelRenderer) Layout(size fyne.Size) {
	r.bg.Resize(size)
	r.bg.Move(fyne.NewPos(0, 0))
	innerPad := r.panel.Theme().Size(theme.SizeNameInnerPadding)
	innerSize := size.Subtract(fyne.NewSquareSize(innerPad * 2))
	r.body.Resize(innerSize)
	r.body.Move(fyne.NewPos(innerPad, innerPad))
	layout.NewStackLayout().Layout(r.body.Objects, innerSize)
	layout.NewVBoxLayout().Layout(r.panel.body.Objects, innerSize)
	r.panel.hoverTipLayer.Resize(innerSize)
	if r.panel.withPreview {
		preview := r.panel.previewDisplay()
		if r.panel.showImage {
			preview.Show()
			r.panel.message.Hide()
		} else {
			preview.Hide()
			r.panel.message.Show()
		}
	}
}

func (r *actionDisplayTooltipPanelRenderer) MinSize() fyne.Size {
	return r.panel.MinSize()
}

func (r *actionDisplayTooltipPanelRenderer) Refresh() {
	th := r.panel.Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	r.bg.FillColor = th.Color(theme.ColorNameOverlayBackground, v)
	r.bg.StrokeColor = th.Color(theme.ColorNameInputBorder, v)
	r.bg.StrokeWidth = th.Size(theme.SizeNameInputBorder)
	if r.panel.withPreview {
		preview := r.panel.previewDisplay()
		if r.panel.showImage {
			preview.Show()
			r.panel.message.Hide()
			if r.panel.caption.Text != "" {
				r.panel.caption.Show()
			} else {
				r.panel.caption.Hide()
			}
		} else {
			preview.Hide()
			r.panel.message.Show()
			r.panel.caption.Hide()
		}
		preview.Refresh()
		r.panel.message.Refresh()
		r.panel.caption.Refresh()
	}
	r.bg.Refresh()
	r.body.Refresh()
}

func (r *actionDisplayTooltipPanelRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.bg, r.body}
}

func (r *actionDisplayTooltipPanelRenderer) Destroy() {}
