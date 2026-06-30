package vision

import (
	"fmt"
	"image"
	"log"

	"gocv.io/x/gocv"
)

type PreprocessOptions struct {
	Grayscale       bool
	Blur            bool
	BlurAmount      int
	Threshold       bool
	MinThreshold    float32
	ThresholdOtsu   bool
	ThresholdInvert bool
	Resize          bool
	ResizeScale     float64
}

func ImageToMatToImagePreprocess(img image.Image, opts PreprocessOptions) image.Image {
	mat, err := preprocessCaptureMat(img, opts)
	if err != nil {
		log.Println(err)
		return nil
	}
	defer mat.Close()
	out, err := mat.ToImage()
	if err != nil {
		log.Println(err)
		return nil
	}
	return out
}

func preprocessCaptureMat(img image.Image, opts PreprocessOptions) (gocv.Mat, error) {
	var out gocv.Mat
	var err error
	WithOpenCV(func() {
		out, err = preprocessCaptureMatLocked(img, opts)
	})
	return out, err
}

func preprocessCaptureMatLocked(img image.Image, opts PreprocessOptions) (gocv.Mat, error) {
	i, err := gocv.ImageToMatRGB(img)
	if err != nil {
		return gocv.Mat{}, err
	}
	if opts.Grayscale {
		gray := gocv.NewMat()
		gocv.CvtColor(i, &gray, gocv.ColorBGRToGray)
		i.Close()
		i = gray
	}
	if opts.Blur && opts.BlurAmount > 0 {
		kernel := opts.BlurAmount
		if kernel%2 == 0 {
			kernel++
		}
		gocv.GaussianBlur(i, &i, image.Point{X: kernel, Y: kernel}, 0, 0, gocv.BorderDefault)
	}
	if opts.Threshold {
		threshType := gocv.ThresholdBinary
		if opts.ThresholdInvert {
			threshType = gocv.ThresholdBinaryInv
		}
		if opts.ThresholdOtsu {
			threshType |= gocv.ThresholdOtsu
		}
		gocv.Threshold(i, &i, opts.MinThreshold, 255, threshType)
		kernel := gocv.GetStructuringElement(gocv.MorphRect, image.Pt(2, 2))
		gocv.MorphologyEx(i, &i, gocv.MorphOpen, kernel)
		kernel.Close()
	}
	if opts.Resize && opts.ResizeScale > 0 && opts.ResizeScale != 1.0 {
		resized := gocv.NewMat()
		interp := gocv.InterpolationDefault
		if opts.ResizeScale > 1.0 {
			interp = gocv.InterpolationCubic
		}
		gocv.Resize(i, &resized, image.Point{}, opts.ResizeScale, opts.ResizeScale, interp)
		i.Close()
		i = resized
	}
	if i.Empty() {
		i.Close()
		return gocv.Mat{}, fmt.Errorf("preprocessing produced empty image")
	}
	return i, nil
}
