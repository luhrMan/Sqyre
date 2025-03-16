package ui

import (
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
)

func (u *Ui) actionSettingsTabs() {
	u.bindVariables()
	//	screen := robotgo.CaptureScreen(0, 0, 2560, 1440)
	//	defer robotgo.FreeBitmap(screen)
	//		mouseMoveDisplay := canvas.NewImageFromImage(robotgo.ToImage(screen))

	// mouseMoveDisplayImage := canvas.NewImageFromFile("./internal/resources/images/full-screen.png")
	// mouseMoveDisplayImage.FillMode = canvas.ImageFillStretch
	// vLine := canvas.NewLine(colornames.Red)
	// hLine := canvas.NewLine(colornames.Red)
	// vLine.StrokeWidth = 2
	// hLine.StrokeWidth = 2
	// mouseMoveDisplayContainer := container.NewBorder(nil, nil, nil, nil, mouseMoveDisplayImage, vLine, hLine)
	//	vLine.Position1 = mouseMoveDisplayContainer.Position()
	// x, _ := u.st.boundMoveX.Get()
	// vLine.Position1.X = float32(x)
	// vLine.Position1.Y = 0
	// vLine.Position2.X = float32(x)
	// vLine.Position2.Y = mouseMoveDisplayImage.Size().Height
	//	vLine.Position1.Y /= 2
	//	hLine.Position1.X /= 2
	//	hLine.Position1.Y /= 2
	//	vLine.Position2.X /= 2
	var (
		waitSettings = container.NewVBox(
			container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("Global Delay"), u.st.boundGlobalDelayEntry, layout.NewSpacer(), widget.NewLabel("ms"))),
			widget.NewLabel("------------------------------------------------------------------------------------"),
			container.NewGridWithColumns(2, container.NewBorder(nil, nil, nil, container.NewHBox(widget.NewLabel("ms")), u.st.boundTimeEntry), u.st.boundTimeSlider),
		)
		moveSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2,
					container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("X:")), nil, u.st.boundMoveXEntry), u.st.boundMoveXSlider,
					container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Y:")), nil, u.st.boundMoveYEntry), u.st.boundMoveYSlider,
					container.NewHBox(layout.NewSpacer(), widget.NewLabel("Spot:")), u.st.boundSpotSelect,
				),
			), nil, nil, nil) //, mouseMoveDisplayContainer)
		clickSettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), u.st.boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
		)
		keySettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), u.st.boundKeySelect, widget.NewLabel("down"), u.st.boundStateToggle, widget.NewLabel("up"), layout.NewSpacer()))
		loopSettings = container.NewVBox(
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), u.st.boundLoopNameEntry),
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("loops:"), u.st.boundCountLabel), u.st.boundCountSlider),
		)
		imageSearchSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("name:")), u.st.boundImageSearchNameEntry),
				container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("search area:")), u.st.boundImageSearchAreaSelect),
				container.NewGridWithColumns(3, container.NewHBox(widget.NewLabel("screen split cols:")), u.st.boundXSplitSlider, u.st.boundXSplitEntry),
			), nil, nil, nil,
			//			u.st.boundImageSearchTargetsTree,
			u.createItemsCheckTree(),
		)

		ocrSettings = container.NewBorder(
			container.NewGridWithColumns(1,
				container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Text Target:")), nil, u.st.boundOCRTargetEntry),
				container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Search Area:")), nil, u.st.boundOCRSearchBoxSelect),
			), nil, nil, nil)
	)

	u.st.tabs.Append(container.NewTabItem("Wait", waitSettings))
	u.st.tabs.Append(container.NewTabItem("Move", moveSettings))
	u.st.tabs.Append(container.NewTabItem("Click", clickSettings))
	u.st.tabs.Append(container.NewTabItem("Key", keySettings))
	u.st.tabs.Append(container.NewTabItem("Loop", loopSettings))
	u.st.tabs.Append(container.NewTabItem("Image", imageSearchSettings))
	u.st.tabs.Append(container.NewTabItem("OCR", ocrSettings))
}
