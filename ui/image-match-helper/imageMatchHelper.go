package imagematchhelper

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/utils"
	"image"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func ImageMatchingHelperWindow() {
	//1. alter template and captured image with threshold and blur
	//2. Find template image within captured image
	//3.
	var (
		win       = fyne.CurrentApp().NewWindow("Image Matching Helper")
		icons     = *assets.BytesToFyneIcons()
		iconBytes = *assets.GetIconBytes()

		templateCanvasImg          = canvas.NewImageFromResource(icons["Ancient Scroll.png"])
		templateCanvasImgContainer = container.NewBorder(nil, nil, nil, nil, templateCanvasImg)
		templateMat                = gocv.NewMat()

		matchImageImg     = robotgo.CaptureImg(0, 0, config.MonitorWidth, config.MonitorHeight)
		matchImgContainer = container.NewBorder(nil, nil, nil, nil, canvas.NewImageFromImage(matchImageImg))
		matchMat, _       = gocv.ImageToMatRGB(matchImageImg)

		tempMat = gocv.NewMat()

		x = binding.NewFloat()
		y = binding.NewFloat()
		w = binding.NewFloat()
		h = binding.NewFloat()

		xSlider = widget.NewSliderWithData(0, float64(config.MonitorWidth)-1, x)
		ySlider = widget.NewSliderWithData(0, float64(config.MonitorHeight)-1, y)
		wSlider = widget.NewSliderWithData(0, float64(config.MonitorWidth), w)
		hSlider = widget.NewSliderWithData(0, float64(config.MonitorHeight), h)

		// xEntry = widget.NewLabelWithData(binding.FloatToStringWithFormat(x, "%.f"))
		// yLabel = widget.NewLabelWithData(binding.FloatToStringWithFormat(y, "%.f"))
		// wLabel = widget.NewLabelWithData(binding.FloatToStringWithFormat(w, "%.f"))
		// hLabel = widget.NewLabelWithData(binding.FloatToStringWithFormat(h, "%.f"))
		xEntry = widget.NewEntryWithData(binding.FloatToStringWithFormat(x, "%.f"))
		yEntry = widget.NewEntryWithData(binding.FloatToStringWithFormat(y, "%.f"))
		wEntry = widget.NewEntryWithData(binding.FloatToStringWithFormat(w, "%.f"))
		hEntry = widget.NewEntryWithData(binding.FloatToStringWithFormat(h, "%.f"))

		blur       = binding.NewFloat()
		blurSlider = widget.NewSliderWithData(0, 20, blur)

		threshold       = binding.NewFloat()
		thresholdSlider = widget.NewSliderWithData(0, 256, threshold)

		matchCheck = widget.NewIcon(theme.CancelIcon())
	)

	templateCanvasImg.FillMode = canvas.ImageFillContain
	templateCanvasImg.SetMinSize(fyne.NewSquareSize(150))
	wSlider.SetValue(float64(config.MonitorWidth))
	hSlider.SetValue(float64(config.MonitorHeight))

	err := gocv.IMDecodeIntoMat(iconBytes["Ancient Scroll.png"], gocv.IMReadColor, &templateMat)
	if err != nil {
		log.Println(err)
	}

	blurSlider.OnChanged = func(f float64) {
		kernel := image.Point{X: int(f), Y: int(f)}
		gocv.GaussianBlur(templateMat, &tempMat, kernel, 0, 0, gocv.BorderDefault)

		iImg, _ := tempMat.ToImage()
		templateCanvasImgContainer.Objects = []fyne.CanvasObject{canvas.NewImageFromImage(iImg)}
		templateCanvasImgContainer.Refresh()
	}
	thresholdSlider.OnChanged = func(f float64) {
		gocv.Threshold(templateMat, &tempMat, float32(f), 255, gocv.ThresholdBinaryInv)

		iImg, _ := tempMat.ToImage()
		templateCanvasImgContainer.Objects = []fyne.CanvasObject{canvas.NewImageFromImage(iImg)}
		templateCanvasImgContainer.Refresh()

		result := gocv.NewMat()

		gocv.MatchTemplate(matchMat, tempMat, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
		matches := utils.GetMatchesFromTemplateMatchResult(result, 0.95, 1)
		// log.Println(matches)
		if len(matches) != 0 {
			matchCheck.SetResource(theme.ConfirmIcon())
			matchCheck.Refresh()
		}
	}
	coordSliderOnChanged := func(f float64) {
		wSlider.Max = float64(config.MonitorWidth) - xSlider.Value
		hSlider.Max = float64(config.MonitorHeight) - ySlider.Value
		if wSlider.Value > wSlider.Max { // ensure the width can't grow greater than the screen
			wSlider.SetValue(wSlider.Max)
		}
		if hSlider.Value > hSlider.Max {
			hSlider.SetValue(hSlider.Max)
		}
		hSlider.Refresh()
		wSlider.Refresh()
		hEntry.Refresh()
		wEntry.Refresh()

		img := robotgo.CaptureImg(int(xSlider.Value), int(ySlider.Value), int(wSlider.Value), int(hSlider.Value))

		matchImgContainer.Objects = []fyne.CanvasObject{canvas.NewImageFromImage(img)}

	}
	xSlider.OnChangeEnded = coordSliderOnChanged
	wSlider.OnChangeEnded = coordSliderOnChanged
	ySlider.OnChangeEnded = coordSliderOnChanged
	hSlider.OnChangeEnded = coordSliderOnChanged

	split := container.NewHSplit(
		container.NewBorder(
			nil,
			widget.NewForm(
				widget.NewFormItem("", matchCheck),
				widget.NewFormItem("Blur Amount", blurSlider),
				widget.NewFormItem("Min Threshold", thresholdSlider),
				widget.NewFormItem("", widget.NewLabel("")),
				widget.NewFormItem("", widget.NewLabel("")),
			),
			nil, nil,
			templateCanvasImgContainer,
		),
		container.NewBorder(
			nil,
			widget.NewForm(
				widget.NewFormItem("Left X", container.NewBorder(nil, nil, xEntry, nil, xSlider)),
				widget.NewFormItem("Width", container.NewBorder(nil, nil, wEntry, nil, wSlider)),
				widget.NewFormItem("Top Y", container.NewBorder(nil, nil, yEntry, nil, ySlider)),
				widget.NewFormItem("Height", container.NewBorder(nil, nil, hEntry, nil, hSlider)),
			),
			nil, nil,
			matchImgContainer,
		),
	)

	win.SetContent(split)
	win.Show()
}
