package main

import (
	"fmt"
	"image"
	"image/color"
	"os"

	"Sqyre/internal/screen"
	"github.com/vcaesar/screenshot"
)

func blackRatio(img image.Image) float64 {
	if img == nil {
		return 1
	}
	b := img.Bounds()
	if b.Empty() {
		return 1
	}
	stepX := b.Dx() / 32
	if stepX < 1 {
		stepX = 1
	}
	stepY := b.Dy() / 32
	if stepY < 1 {
		stepY = 1
	}
	var samples int
	var black int
	for y := 0; y < b.Dy(); y += stepY {
		for x := 0; x < b.Dx(); x += stepX {
			c := color.RGBAModel.Convert(img.At(b.Min.X+x, b.Min.Y+y)).(color.RGBA)
			samples++
			if c.R <= 2 && c.G <= 2 && c.B <= 2 {
				black++
			}
		}
	}
	if samples == 0 {
		return 1
	}
	return float64(black) / float64(samples)
}

func main() {
	fmt.Println("DISPLAY =", os.Getenv("DISPLAY"))
	fmt.Println("WAYLAND_DISPLAY =", os.Getenv("WAYLAND_DISPLAY"))
	fmt.Println("XDG_SESSION_TYPE =", os.Getenv("XDG_SESSION_TYPE"))
	fmt.Println("screenshot.NumActiveDisplays =", screenshot.NumActiveDisplays())
	fmt.Println("screen.NumDisplays =", screen.NumDisplays())

	n := screen.NumDisplays()
	for i := 0; i < n; i++ {
		abs := screen.DisplayBoundsAbs(i)
		shot := screenshot.GetDisplayBounds(i)
		fmt.Printf("display[%d] abs=%v screenshot=%v\n", i, abs, shot)

		simg, serr := screenshot.CaptureDisplay(i)
		if serr != nil {
			fmt.Printf("  screenshot.CaptureDisplay err=%v\n", serr)
		} else {
			fmt.Printf("  screenshot.CaptureDisplay bounds=%v black_ratio=%.3f\n", simg.Bounds(), blackRatio(simg))
		}
	}
}
