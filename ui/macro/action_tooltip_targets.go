package macro

import (
	"fmt"
	"slices"

	"Sqyre/internal/assets"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/uiutil"
	"Sqyre/ui/actiondisplay"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

const (
	targetRemoveIconSize                   float32 = 10
	imageSearchTargetGlyphSize                     = 20
	imageSearchTargetIconRenderSize                = 32
)

func imageSearchTargetIconSize() fyne.Size {
	return fyne.NewSquareSize(imageSearchTargetIconRenderSize)
}

func imageSearchTargetsFromNode(node actions.ActionInterface) []string {
	is, ok := node.(*actions.ImageSearch)
	if !ok || len(is.Targets) == 0 {
		return nil
	}
	return is.Targets
}

func imageSearchTargetIcon(target string, size fyne.Size) *canvas.Image {
	path := uiutil.IconPathForTarget(target)
	if path == "" {
		return nil
	}
	res := assets.GetFyneResource(path)
	if res == nil {
		return nil
	}
	img := canvas.NewImageFromResource(res)
	img.SetMinSize(size)
	img.FillMode = canvas.ImageFillContain
	return img
}

// imageSearchTargetIconCell renders a target icon at a fixed size so tree rows and
// tooltips match regardless of the PNG's intrinsic dimensions.
type imageSearchTargetIconCell struct {
	widget.BaseWidget

	size fyne.Size
	icon *canvas.Image
}

func newImageSearchTargetIconCell(target string, size fyne.Size) *imageSearchTargetIconCell {
	img := imageSearchTargetIcon(target, size)
	if img == nil {
		return nil
	}
	c := &imageSearchTargetIconCell{size: size, icon: img}
	c.ExtendBaseWidget(c)
	return c
}

func (c *imageSearchTargetIconCell) MinSize() fyne.Size {
	return c.size
}

func (c *imageSearchTargetIconCell) CreateRenderer() fyne.WidgetRenderer {
	return &imageSearchTargetIconCellRenderer{cell: c}
}

type imageSearchTargetIconCellRenderer struct {
	cell *imageSearchTargetIconCell
}

func (r *imageSearchTargetIconCellRenderer) Layout(size fyne.Size) {
	r.cell.icon.Resize(size)
	r.cell.icon.Move(fyne.NewPos(0, 0))
}

func (r *imageSearchTargetIconCellRenderer) MinSize() fyne.Size {
	return r.cell.size
}

func (r *imageSearchTargetIconCellRenderer) Refresh() {
	r.cell.icon.Refresh()
}

func (r *imageSearchTargetIconCellRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.cell.icon}
}

func (r *imageSearchTargetIconCellRenderer) Destroy() {}

func imageSearchItemCountBadge(count int) fyne.CanvasObject {
	glyphSize := fyne.NewSquareSize(imageSearchTargetGlyphSize)
	glyph := canvas.NewImageFromResource(assets.TargetIcon)
	glyph.SetMinSize(glyphSize)
	glyph.FillMode = canvas.ImageFillContain
	content := container.NewHBox(
		actiondisplay.NewPillText(fmt.Sprintf("%d", count)),
		glyph,
	)
	return actiondisplay.PillChrome(content, "imagesearch")
}

func imageSearchRowTargetIcons(targets []string) fyne.CanvasObject {
	if len(targets) == 0 {
		return nil
	}
	box := container.NewHBox()
	box.Add(imageSearchItemCountBadge(len(targets)))
	size := imageSearchTargetIconSize()
	for _, target := range targets {
		if cell := newImageSearchTargetIconCell(target, size); cell != nil {
			box.Add(cell)
		}
	}
	return box
}

func imageSearchTargetIconsSection(count int, icons fyne.CanvasObject) fyne.CanvasObject {
	if count <= 0 && icons == nil {
		return nil
	}
	box := container.New(kxlayout.NewRowWrapLayout())
	box.Add(imageSearchItemCountBadge(count))
	if icons != nil {
		if c, ok := icons.(*fyne.Container); ok {
			box.Objects = append(box.Objects, c.Objects...)
		} else {
			box.Add(icons)
		}
	}
	return wrapTooltipSection(box)
}

func imageSearchTargetIconsView(targets []string) fyne.CanvasObject {
	if len(targets) == 0 {
		return nil
	}
	size := imageSearchTargetIconSize()
	icons := container.New(kxlayout.NewRowWrapLayout())
	for _, target := range targets {
		if cell := newImageSearchTargetIconCell(target, size); cell != nil {
			icons.Add(cell)
		}
	}
	return imageSearchTargetIconsSection(len(targets), icons)
}

type imageSearchTargetRemove struct {
	widget.BaseWidget

	icon     *canvas.Image
	onRemove func()
}

func newImageSearchTargetRemove(onRemove func()) *imageSearchTargetRemove {
	r := &imageSearchTargetRemove{onRemove: onRemove}
	r.icon = canvas.NewImageFromResource(theme.CancelIcon())
	r.icon.SetMinSize(fyne.NewSquareSize(targetRemoveIconSize))
	r.icon.FillMode = canvas.ImageFillContain
	r.ExtendBaseWidget(r)
	return r
}

func (r *imageSearchTargetRemove) MinSize() fyne.Size {
	return fyne.NewSquareSize(targetRemoveIconSize)
}

func (r *imageSearchTargetRemove) Tapped(*fyne.PointEvent) {
	if r.onRemove != nil {
		r.onRemove()
	}
}

func (r *imageSearchTargetRemove) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(r.icon)
}

var _ fyne.Tappable = (*imageSearchTargetRemove)(nil)

// imageSearchTargetEditCell shows target icons at imageSearchTargetIconRenderSize; a small tappable X is overlaid.
type imageSearchTargetEditCell struct {
	widget.BaseWidget

	size   fyne.Size
	icon   *canvas.Image
	remove *imageSearchTargetRemove
}

func newImageSearchTargetEditCell(icon *canvas.Image, onRemove func()) *imageSearchTargetEditCell {
	size := imageSearchTargetIconSize()
	c := &imageSearchTargetEditCell{
		size: size,
		icon: icon,
	}
	c.remove = newImageSearchTargetRemove(onRemove)
	c.ExtendBaseWidget(c)
	return c
}

func (c *imageSearchTargetEditCell) MinSize() fyne.Size {
	return c.size
}

func (c *imageSearchTargetEditCell) CreateRenderer() fyne.WidgetRenderer {
	return &imageSearchTargetEditCellRenderer{cell: c}
}

type imageSearchTargetEditCellRenderer struct {
	cell *imageSearchTargetEditCell
}

func (r *imageSearchTargetEditCellRenderer) Layout(size fyne.Size) {
	r.cell.icon.Resize(size)
	r.cell.icon.Move(fyne.NewPos(0, 0))

	btnSize := fyne.NewSquareSize(targetRemoveIconSize)
	x := size.Width - btnSize.Width
	if x < 0 {
		x = 0
	}
	r.cell.remove.Resize(btnSize)
	r.cell.remove.Move(fyne.NewPos(x, 0))
}

func (r *imageSearchTargetEditCellRenderer) MinSize() fyne.Size {
	return r.cell.size
}

func (r *imageSearchTargetEditCellRenderer) Refresh() {
	r.cell.icon.Refresh()
	r.cell.remove.Refresh()
}

func (r *imageSearchTargetEditCellRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.cell.icon, r.cell.remove}
}

func (r *imageSearchTargetEditCellRenderer) Destroy() {}

func buildImageSearchTargetEdit(a *actions.ImageSearch, owner *actionDisplayTooltipHover) (fyne.CanvasObject, func() error) {
	temp := slices.Clone(a.Targets)
	section := container.New(kxlayout.NewRowWrapLayout())
	size := imageSearchTargetIconSize()

	var rebuild func()
	rebuild = func() {
		section.Objects = nil
		section.Add(imageSearchItemCountBadge(len(temp)))
		for _, target := range temp {
			img := imageSearchTargetIcon(target, size)
			if img == nil {
				img = canvas.NewImageFromResource(assets.AppIcon)
				img.SetMinSize(size)
				img.FillMode = canvas.ImageFillContain
			}
			t := target
			section.Add(newImageSearchTargetEditCell(img, func() {
				if i := slices.Index(temp, t); i >= 0 {
					temp = slices.Delete(temp, i, i+1)
					rebuild()
					if owner != nil {
						owner.relayoutTooltip()
					}
				}
			}))
		}
		section.Refresh()
	}
	rebuild()

	apply := func() error {
		a.Targets = slices.Clone(temp)
		slices.Sort(a.Targets)
		return nil
	}
	return wrapTooltipSection(section), apply
}
