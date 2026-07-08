package macro

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/vision"
	"Sqyre/ui/custom_widgets"
)

func actionPreviewLoader(node actions.ActionInterface) custom_widgets.PreviewTooltipLoad {
	ref, ok := coordinateRefForPreview(node)
	if !ok || ref.IsEmpty() {
		return nil
	}
	return previewLoaderForRef(node, ref)
}

func coordinateRefForPreview(node actions.ActionInterface) (actions.CoordinateRef, bool) {
	switch a := node.(type) {
	case *actions.Move:
		return a.Point, true
	case *actions.ImageSearch:
		return a.SearchArea, true
	case *actions.Ocr:
		return a.SearchArea, true
	case *actions.FindPixel:
		return a.SearchArea, true
	case *actions.SemanticSearch:
		return a.SearchArea, true
	default:
		return "", false
	}
}

func previewLoaderForRef(node actions.ActionInterface, ref actions.CoordinateRef) custom_widgets.PreviewTooltipLoad {
	if ref.IsEmpty() {
		return nil
	}
	switch node.(type) {
	case *actions.Move:
		return func() (custom_widgets.PreviewTooltipResult, error) {
			img, caption, err := vision.PointPreviewTooltipForRef(ref)
			return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, err
		}
	case *actions.ImageSearch, *actions.Ocr, *actions.FindPixel, *actions.SemanticSearch:
		return func() (custom_widgets.PreviewTooltipResult, error) {
			img, caption, err := vision.SearchAreaPreviewTooltipForRef(ref)
			return custom_widgets.PreviewTooltipResult{Image: img, Caption: caption}, err
		}
	default:
		return nil
	}
}
