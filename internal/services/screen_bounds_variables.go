package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
)

// ApplyScreenBoundsVariables sets virtual-desktop bounds on the macro variable store
// so search areas can use ${screenMinX}, ${screenMaxX}, ${screenMidX}, etc.
// Right/bottom edges match image.Rectangle.Max (exclusive), consistent with CaptureImg width/height.
func ApplyScreenBoundsVariables(vs *models.VariableStore) {
	if vs == nil {
		return
	}
	vb := screen.VirtualBounds()
	vs.Set("screenMinX", vb.Min.X)
	vs.Set("screenMinY", vb.Min.Y)
	vs.Set("screenMaxX", vb.Max.X)
	vs.Set("screenMaxY", vb.Max.Y)
	vs.Set("screenWidth", vb.Dx())
	vs.Set("screenHeight", vb.Dy())
	vs.Set("screenMidX", vb.Min.X+vb.Dx()/2)
	vs.Set("screenMidY", vb.Min.Y+vb.Dy()/2)
}

// ResolveSearchAreaCoordsForPreview resolves search area corners using only built-in screen variables
// (same values as at macro start). Use for editor preview when coordinates may be ${...} strings.
func ResolveSearchAreaCoordsForPreview(leftX, topY, rightX, bottomY any) (int, int, int, int, error) {
	m := &models.Macro{Name: "", Variables: models.NewVariableStore()}
	ApplyScreenBoundsVariables(m.Variables)
	return ResolveSearchAreaCoords(leftX, topY, rightX, bottomY, m)
}
