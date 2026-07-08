package detector

import (
	"image"
	"image/color"
)

// letterboxResult holds a square RGB tensor in NCHW order plus mapping metadata.
type letterboxResult struct {
	data      []float32
	size      int
	scale     float32
	padX      int
	padY      int
	srcWidth  int
	srcHeight int
}

// letterboxResize scales frame into a square canvas with gray padding (114/255).
// Returns float32 RGB values normalized to [0,1] in planar CHW layout.
func letterboxResize(frame image.Image, inputSize int) letterboxResult {
	bounds := frame.Bounds()
	srcW := bounds.Dx()
	srcH := bounds.Dy()
	if srcW <= 0 || srcH <= 0 {
		return letterboxResult{size: inputSize, data: make([]float32, 3*inputSize*inputSize)}
	}

	scale := float32(inputSize) / float32(max(srcW, srcH))
	newW := int(float32(srcW) * scale)
	newH := int(float32(srcH) * scale)
	padX := (inputSize - newW) / 2
	padY := (inputSize - newH) / 2

	data := make([]float32, 3*inputSize*inputSize)
	const padVal = 114.0 / 255.0
	for i := range data {
		data[i] = padVal
	}

	for y := 0; y < newH; y++ {
		srcY := bounds.Min.Y + int(float32(y)/scale+0.5)
		for x := 0; x < newW; x++ {
			srcX := bounds.Min.X + int(float32(x)/scale+0.5)
			r, g, b, _ := frame.At(srcX, srcY).RGBA()
			dstX := padX + x
			dstY := padY + y
			setCHW(data, inputSize, dstX, dstY, color.RGBA{
				R: uint8(r >> 8),
				G: uint8(g >> 8),
				B: uint8(b >> 8),
			})
		}
	}

	return letterboxResult{
		data:      data,
		size:      inputSize,
		scale:     scale,
		padX:      padX,
		padY:      padY,
		srcWidth:  srcW,
		srcHeight: srcH,
	}
}

func setCHW(data []float32, size, x, y int, c color.RGBA) {
	if x < 0 || y < 0 || x >= size || y >= size {
		return
	}
	offset := y*size + x
	plane := size * size
	data[offset] = float32(c.R) / 255
	data[plane+offset] = float32(c.G) / 255
	data[2*plane+offset] = float32(c.B) / 255
}

// mapBoxFromLetterbox converts a center-size box in letterbox space to source image coords.
func (lb letterboxResult) mapBox(cx, cy, w, h float32) image.Rectangle {
	x1 := (cx - w/2 - float32(lb.padX)) / lb.scale
	y1 := (cy - h/2 - float32(lb.padY)) / lb.scale
	x2 := (cx + w/2 - float32(lb.padX)) / lb.scale
	y2 := (cy + h/2 - float32(lb.padY)) / lb.scale

	x1 = clampf(x1, 0, float32(lb.srcWidth))
	y1 = clampf(y1, 0, float32(lb.srcHeight))
	x2 = clampf(x2, 0, float32(lb.srcWidth))
	y2 = clampf(y2, 0, float32(lb.srcHeight))

	return image.Rect(int(x1), int(y1), int(x2), int(y2))
}

func clampf(v, lo, hi float32) float32 {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}
	return v
}
