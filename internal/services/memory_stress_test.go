//go:build matprofile

package services

import (
	"image"
	"image/color"
	"runtime"
	"testing"

	"gocv.io/x/gocv"
)

func syntheticCapture(w, h int) image.Image {
	img := image.NewRGBA(image.Rect(0, 0, w, h))
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			img.Set(x, y, color.RGBA{uint8(x % 256), uint8(y % 256), 128, 255})
		}
	}
	return img
}

func TestImagePreprocessDoesNotLeakMats(t *testing.T) {
	img := syntheticCapture(800, 600)
	before := gocv.MatProfile.Count()
	for i := 0; i < 100; i++ {
		out := ImageToMatToImagePreprocess(img, PreprocessOptions{
			Grayscale:     true,
			Blur:            true,
			BlurAmount:      5,
			Threshold:       true,
			MinThreshold:    127,
			ThresholdOtsu:   true,
			Resize:          true,
			ResizeScale:     1.5,
		})
		if out == nil {
			t.Fatalf("preprocess returned nil at iter %d", i)
		}
	}
	runtime.GC()
	after := gocv.MatProfile.Count()
	if after != before {
		t.Fatalf("MatProfile grew by %d during preprocess loop", after-before)
	}
}

func TestTemplateMatchDoesNotLeakMats(t *testing.T) {
	search := gocv.NewMatWithSize(400, 400, gocv.MatTypeCV8UC3)
	defer search.Close()
	search.SetTo(gocv.NewScalar(128, 128, 128, 0))

	template := gocv.NewMatWithSize(32, 32, gocv.MatTypeCV8UC3)
	defer template.Close()
	template.SetTo(gocv.NewScalar(200, 100, 50, 0))

	imask := gocv.NewMat()
	defer imask.Close()
	tmask := gocv.NewMat()
	defer tmask.Close()
	cmask := gocv.NewMat()
	defer cmask.Close()

	before := gocv.MatProfile.Count()
	for i := 0; i < 200; i++ {
		_ = FindTemplateMatches(search, template, imask, tmask, cmask, 0.5, 5)
	}
	runtime.GC()
	after := gocv.MatProfile.Count()
	if after != before {
		t.Fatalf("MatProfile grew by %d during template match loop (before=%d after=%d)", after-before, before, after)
	}
}

func TestBlurForSearchDoesNotLeakMats(t *testing.T) {
	img := gocv.NewMatWithSize(600, 800, gocv.MatTypeCV8UC3)
	defer img.Close()
	img.SetTo(gocv.NewScalar(64, 64, 64, 0))

	before := gocv.MatProfile.Count()
	for i := 0; i < 100; i++ {
		blurred := blurForSearch(img, 5)
		blurred.Close()
	}
	runtime.GC()
	after := gocv.MatProfile.Count()
	if after != before {
		t.Fatalf("MatProfile grew by %d during blurForSearch loop", after-before)
	}
}

func TestImageToMatRGBDoesNotLeakMats(t *testing.T) {
	img := syntheticCapture(1920, 1080)
	before := gocv.MatProfile.Count()
	for i := 0; i < 50; i++ {
		mat, err := gocv.ImageToMatRGB(img)
		if err != nil {
			t.Fatal(err)
		}
		mat.Close()
	}
	runtime.GC()
	after := gocv.MatProfile.Count()
	if after != before {
		t.Fatalf("MatProfile grew by %d during ImageToMatRGB loop", after-before)
	}
}

func TestOCRPathDoesNotLeakMats(t *testing.T) {
	img := syntheticCapture(640, 480)
	before := gocv.MatProfile.Count()
	for i := 0; i < 20; i++ {
		_, _, err := ocrImageWithBoxes(img)
		if err != nil {
			t.Fatal(err)
		}
		releaseTessClientImage()
	}
	runtime.GC()
	after := gocv.MatProfile.Count()
	if after != before {
		t.Fatalf("MatProfile grew by %d during OCR loop", after-before)
	}
}

func TestOCRPathHeapGrowth(t *testing.T) {
	img := syntheticCapture(1920, 1080)
	var m1, m2 runtime.MemStats
	runtime.GC()
	runtime.ReadMemStats(&m1)
	for i := 0; i < 100; i++ {
		_, _, err := ocrImageWithBoxes(img)
		if err != nil {
			t.Fatal(err)
		}
		releaseTessClientImage()
	}
	runtime.GC()
	runtime.ReadMemStats(&m2)
	growthMB := int64(m2.Alloc-m1.Alloc) / 1024 / 1024
	t.Logf("OCR 100x 1920x1080 heap growth: %d MB (sys %d -> %d)", growthMB, m1.Sys/1024/1024, m2.Sys/1024/1024)
	if growthMB > 50 {
		t.Fatalf("heap grew %d MB, likely leak", growthMB)
	}
}
