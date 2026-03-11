//go:build android

package desktop

import (
	"image"
	"image/color"
	"image/draw"
	"image/png"
	"os"
)

func MousePosition() (x, y int) { return 0, 0 }

func MilliSleep(ms int) {}

func ScreenSize() (w, h int) { return 1080, 1920 }

func DisplayCount() int { return 1 }

func DisplayBounds(displayIndex int) (x, y, w, h int) {
	return 0, 0, 1080, 1920
}

func CaptureRegion(x, y, w, h int) (image.Image, error) {
	// No screen capture on Android; return a small placeholder.
	img := image.NewRGBA(image.Rect(0, 0, 1, 1))
	img.Set(0, 0, color.Black)
	return img, nil
}

func SavePNG(img image.Image, path string) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return png.Encode(f, img)
}

func GetPixelColor(x, y int) string { return "000000" }

func DrawCrosshairOnImage(img image.Image, px, py int) (image.Image, error) {
	bounds := img.Bounds()
	out := image.NewRGBA(bounds)
	draw.Draw(out, bounds, img, bounds.Min, draw.Src)
	red := color.RGBA{R: 255, A: 255}
	for dx := -2; dx <= 2; dx++ {
		for dy := -8; dy <= 8; dy++ {
			if c := (image.Point{X: px + dx, Y: py + dy}); c.In(bounds) {
				out.SetRGBA(c.X, c.Y, red)
			}
			if c := (image.Point{X: px + dy, Y: py + dx}); c.In(bounds) {
				out.SetRGBA(c.X, c.Y, red)
			}
		}
	}
	return out, nil
}

func DrawRectOnImage(img image.Image, r image.Rectangle) (image.Image, error) {
	bounds := img.Bounds()
	out := image.NewRGBA(bounds)
	draw.Draw(out, bounds, img, bounds.Min, draw.Src)
	red := color.RGBA{R: 255, A: 255}
	clip := r.Intersect(bounds)
	for x := clip.Min.X; x < clip.Max.X; x++ {
		if clip.Min.Y >= bounds.Min.Y {
			out.SetRGBA(x, clip.Min.Y, red)
		}
		if clip.Max.Y-1 < bounds.Max.Y {
			out.SetRGBA(x, clip.Max.Y-1, red)
		}
	}
	for y := clip.Min.Y; y < clip.Max.Y; y++ {
		if clip.Min.X >= bounds.Min.X {
			out.SetRGBA(clip.Min.X, y, red)
		}
		if clip.Max.X-1 < bounds.Max.X {
			out.SetRGBA(clip.Max.X-1, y, red)
		}
	}
	return out, nil
}

func RegisterMouseDown(cb func(x, y, button int)) func() {
	return func() {}
}

func SetMouseKeySleep(mouseMs, keyMs int) {}
