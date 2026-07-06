package macro

import (
	"Sqyre/internal/models/actions"
)

type coordinateRefBinding struct {
	ref actions.CoordinateRef
	set func(actions.CoordinateRef)
}

func actionCoordinateBinding(node actions.ActionInterface) (coordinateRefBinding, bool) {
	switch a := node.(type) {
	case *actions.Move:
		return coordinateRefBinding{ref: a.Point, set: func(r actions.CoordinateRef) { a.Point = r }}, true
	case *actions.ImageSearch:
		return coordinateRefBinding{ref: a.SearchArea, set: func(r actions.CoordinateRef) { a.SearchArea = r }}, true
	case *actions.Ocr:
		return coordinateRefBinding{ref: a.SearchArea, set: func(r actions.CoordinateRef) { a.SearchArea = r }}, true
	case *actions.FindPixel:
		return coordinateRefBinding{ref: a.SearchArea, set: func(r actions.CoordinateRef) { a.SearchArea = r }}, true
	default:
		return coordinateRefBinding{}, false
	}
}

func actionUsesPointPicker(node actions.ActionInterface) bool {
	_, ok := node.(*actions.Move)
	return ok
}
