package macro

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/vision"
	"Sqyre/ui/custom_widgets"
)

func actionPreviewLoader(node actions.ActionInterface) custom_widgets.PreviewTooltipLoad {
	switch a := node.(type) {
	case *actions.Move:
		if a.Point.IsEmpty() {
			return nil
		}
		ref := a.Point
		return func() (custom_widgets.PreviewTooltipResult, error) {
			img, caption, err := vision.PointPreviewTooltipForRef(ref)
			return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, err
		}
	case *actions.ImageSearch:
		if a.SearchArea.IsEmpty() {
			return nil
		}
		ref := a.SearchArea
		return func() (custom_widgets.PreviewTooltipResult, error) {
			img, caption, err := vision.SearchAreaPreviewTooltipForRef(ref)
			return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, err
		}
	case *actions.Ocr:
		if a.SearchArea.IsEmpty() {
			return nil
		}
		ref := a.SearchArea
		return func() (custom_widgets.PreviewTooltipResult, error) {
			img, caption, err := vision.SearchAreaPreviewTooltipForRef(ref)
			return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, err
		}
	case *actions.FindPixel:
		if a.SearchArea.IsEmpty() {
			return nil
		}
		ref := a.SearchArea
		return func() (custom_widgets.PreviewTooltipResult, error) {
			img, caption, err := vision.SearchAreaPreviewTooltipForRef(ref)
			return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, err
		}
	default:
		return nil
	}
}
