//go:build gocv

package main

import (
	"fmt"
	"image"
	"image/color"

	"Sqyre/internal/services"
	"Sqyre/internal/vision"

	"gocv.io/x/gocv"
)

func runGocvSection(name string, iterations uint64) sectionResult {
	switch name {
	case "match_direct":
		return benchMatch("match_direct", iterations, 96, 72, 12, 10,
			"gocv MatchTemplate TM_CCOEFF_NORMED 96x72 / 12x10")
	case "match_fft":
		// Go always uses OpenCV MatchTemplate (no separate FFT path); same sizes as Rust FFT case.
		return benchMatch("match_fft", iterations, 320, 240, 32, 24,
			"gocv MatchTemplate TM_CCOEFF_NORMED 320x240 / 32x24 (OpenCV path; Rust uses FFT)")
	case "match_multi_variant":
		return benchMatchMulti(iterations)
	case "search_prep":
		return benchSearchPrep(iterations)
	case "find_pixels":
		return benchFindPixels(iterations)
	case "ocr_preprocess":
		return benchOcrPreprocess(iterations)
	default:
		return failed(name, "not a gocv section")
	}
}

func xorshift(seed *uint64) uint8 {
	*seed ^= *seed << 13
	*seed ^= *seed >> 7
	*seed ^= *seed << 17
	return uint8(*seed % 256)
}

func randomRGBA(w, h int, seed uint64) *image.RGBA {
	img := image.NewRGBA(image.Rect(0, 0, w, h))
	s := seed
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			img.SetRGBA(x, y, color.RGBA{
				R: xorshift(&s), G: xorshift(&s), B: xorshift(&s), A: 255,
			})
		}
	}
	return img
}

func matFromRGBA(img *image.RGBA) (gocv.Mat, error) {
	return gocv.ImageToMatRGB(img)
}

func benchMatch(name string, iterations uint64, sw, sh, tw, th int, notes string) sectionResult {
	searchImg := randomRGBA(sw, sh, 1)
	templImg := randomRGBA(tw, th, 2)
	var search, templ gocv.Mat
	var err error
	vision.WithOpenCV(func() {
		search, err = matFromRGBA(searchImg)
		if err != nil {
			return
		}
		templ, err = matFromRGBA(templImg)
	})
	if err != nil {
		return failed(name, err.Error())
	}
	defer search.Close()
	defer templ.Close()

	empty := gocv.NewMat()
	defer empty.Close()

	return timed(name, iterations, notes, func() error {
		var runErr error
		vision.WithOpenCV(func() {
			_ = services.FindTemplateMatches(search, templ, empty, empty, empty, 0.8, 0)
		})
		return runErr
	})
}

func benchMatchMulti(iterations uint64) sectionResult {
	searchImg := randomRGBA(160, 120, 10)
	var search gocv.Mat
	var err error
	vision.WithOpenCV(func() {
		search, err = matFromRGBA(searchImg)
	})
	if err != nil {
		return failed("match_multi_variant", err.Error())
	}
	defer search.Close()

	type pair struct{ tmpl gocv.Mat }
	variants := make([]pair, 8)
	for i := range variants {
		img := randomRGBA(16, 12, uint64(100+i))
		vision.WithOpenCV(func() {
			variants[i].tmpl, err = matFromRGBA(img)
		})
		if err != nil {
			return failed("match_multi_variant", err.Error())
		}
	}
	defer func() {
		for i := range variants {
			variants[i].tmpl.Close()
		}
	}()
	empty := gocv.NewMat()
	defer empty.Close()

	return timed("match_multi_variant", iterations,
		"8× FindTemplateMatches on shared 160x120 search", func() error {
			vision.WithOpenCV(func() {
				for i := range variants {
					_ = services.FindTemplateMatches(search, variants[i].tmpl, empty, empty, empty, 0.8, 0)
				}
			})
			return nil
		})
}

func benchSearchPrep(iterations uint64) sectionResult {
	// Go has no SearchPrep; approximate with ImageToMatRGB + optional blur(0).
	img := randomRGBA(640, 480, 7)
	return timed("search_prep", iterations,
		"gocv ImageToMatRGB 640x480 (closest to Rust prepare_search)", func() error {
			var err error
			vision.WithOpenCV(func() {
				mat, e := matFromRGBA(img)
				if e != nil {
					err = e
					return
				}
				mat.Close()
			})
			return err
		})
}

func benchFindPixels(iterations uint64) sectionResult {
	img := randomRGBA(640, 480, 9)
	img.SetRGBA(637, 478, color.RGBA{R: 0xcc, G: 0x33, B: 0x99, A: 255})
	return timed("find_pixels", iterations,
		"services.FindPixelInRGBAForBench (real findPixelInRGBA)", func() error {
			_, _, ok := services.FindPixelInRGBAForBench(img, 0xcc, 0x33, 0x99, 0)
			if !ok {
				return fmt.Errorf("pixel not found")
			}
			return nil
		})
}

func benchOcrPreprocess(iterations uint64) sectionResult {
	img := randomRGBA(320, 80, 11)
	opts := vision.PreprocessOptions{
		Grayscale: true, Blur: true, BlurAmount: 1,
		Threshold: true, ThresholdOtsu: true,
	}
	return timed("ocr_preprocess", iterations,
		"vision.ImageToMatToImagePreprocess gray+blur+otsu", func() error {
			out := vision.ImageToMatToImagePreprocess(img, opts)
			if out == nil {
				return fmt.Errorf("preprocess returned nil")
			}
			return nil
		})
}
