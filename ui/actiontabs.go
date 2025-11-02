package ui

import (
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

const (
	WaitTab = iota
	MoveTab
	ClickTab
	KeyTab
	LoopTab
	ImageSearchTab
	OcrTab
)

type ActionTabs struct {
	*container.AppTabs
	BoundWait  binding.Struct
	BoundMove  binding.Struct
	BoundPoint binding.Struct
	BoundClick binding.Struct
	BoundKey   binding.Struct

	BoundLoop          binding.Struct
	BoundLoopAA        binding.Struct
	BoundImageSearch   binding.Struct
	BoundImageSearchAA binding.Struct
	BoundImageSearchSA binding.Struct
	BoundOcr           binding.Struct
	BoundOcrAA         binding.Struct
	BoundOcrSA         binding.Struct

	BoundTimeSlider *widget.Slider
	BoundTimeEntry  *widget.Entry

	PointsAccordion *widget.Accordion

	BoundButtonToggle *custom_widgets.Toggle

	BoundKeySelect   *widget.Select
	BoundStateToggle *custom_widgets.Toggle

	BoundLoopNameEntry *widget.Entry
	BoundCountSlider   *widget.Slider
	BoundCountLabel    *widget.Label

	BoundImageSearchNameEntry      *widget.Entry
	BoundImageSearchColSplitSlider *ttwidget.Slider
	BoundImageSearchRowSplitSlider *ttwidget.Slider
	ImageSearchSAAccordion         *widget.Accordion
	ImageSearchItemsAccordion      *widget.Accordion

	boundOCRTargetEntry *widget.Entry
	OCRSAAccordion      *widget.Accordion
}

func newActionTabs() *ActionTabs {
	return &ActionTabs{
		AppTabs: new(container.AppTabs),

		BoundTimeSlider:   widget.NewSliderWithData(0.0, 1000.0, binding.NewFloat()),
		BoundTimeEntry:    new(widget.Entry),
		PointsAccordion:   widget.NewAccordion(),
		BoundButtonToggle: custom_widgets.NewToggleWithData(binding.NewBool()),
		BoundKeySelect:    widget.NewSelectWithData([]string{"ctrl", "alt", "shift"}, binding.NewString()),
		BoundStateToggle:  custom_widgets.NewToggleWithData(binding.NewBool()),

		BoundLoopNameEntry: widget.NewEntryWithData(binding.NewString()),
		BoundCountSlider:   widget.NewSliderWithData(1, 10, binding.IntToFloat(binding.NewInt())),
		BoundCountLabel:    widget.NewLabelWithData(binding.NewString()),

		BoundImageSearchNameEntry:      widget.NewEntryWithData(binding.NewString()),
		BoundImageSearchColSplitSlider: ttwidget.NewSlider(0, 100),
		BoundImageSearchRowSplitSlider: ttwidget.NewSlider(0, 100),

		ImageSearchSAAccordion:    widget.NewAccordion(),
		ImageSearchItemsAccordion: widget.NewAccordion(),

		boundOCRTargetEntry: widget.NewEntryWithData(binding.NewString()),
		OCRSAAccordion:      widget.NewAccordion(),
	}
}

func (u *Ui) constructActionTabs() *ActionTabs {
	u.ActionTabs.constructWaitTab()
	u.ActionTabs.constructMoveTab()
	u.ActionTabs.constructClickTab()
	u.ActionTabs.constructKeyTab()
	u.ActionTabs.constructLoopTab()
	u.ActionTabs.constructImageSearchTab()
	u.ActionTabs.constructOcrTab()
	return u.ActionTabs
}

func (at *ActionTabs) constructWaitTab() {
	gdfi := widget.NewFormItem("delay", GetUi().Mui.MTabs.BoundGlobalDelayEntry)
	gdfi.HintText = "keyboard & mouse global delay (ms)"

	waitSettings :=
		widget.NewForm(
			widget.NewFormItem("ms", container.NewGridWithColumns(2,
				at.BoundTimeEntry, at.BoundTimeSlider,
			)),
			gdfi,
		)
	at.Append(container.NewTabItem("Wait", waitSettings))
}
func (at *ActionTabs) constructMoveTab() {
	moveSettings :=
		container.NewBorder(
			nil, nil, nil, nil,
			at.PointsAccordion,
		)
	at.Append(container.NewTabItem("Move", moveSettings))
}

func (at *ActionTabs) constructClickTab() {
	clickSettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				widget.NewLabel("left"),
				at.BoundButtonToggle,
				widget.NewLabel("right"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Click", clickSettings))
}

func (at *ActionTabs) constructKeyTab() {
	keySettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				at.BoundKeySelect,
				widget.NewLabel("up"),
				at.BoundStateToggle,
				widget.NewLabel("down"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Key", keySettings))
}

func (at *ActionTabs) constructLoopTab() {
	loopSettings :=
		widget.NewForm(
			widget.NewFormItem("Name:", at.BoundLoopNameEntry),
			widget.NewFormItem("Loops:", container.NewBorder(
				nil, nil, at.BoundCountLabel, nil, at.BoundCountSlider,
			)),
		)
	at.Append(container.NewTabItem("Loop", loopSettings))
}

func (at *ActionTabs) constructImageSearchTab() {
	// colIcon := ttwidget.NewIcon(theme.NewDisabledResource(theme.MoreVerticalIcon()))
	// colIcon.SetToolTip("columns split")
	// rowIcon := ttwidget.NewIcon(theme.NewDisabledResource(theme.MoreHorizontalIcon()))
	// rowIcon.SetToolTip("rows split")
	// at.BoundImageSearchColSplitSlider.OnChanged = func(f float64) {
	// 	at.BoundImageSearchColSplitSlider.SetToolTip(strconv.FormatFloat(f, 'f', -1, 64))
	// }
	imageSearchSettings :=
		container.NewScroll(
			container.NewBorder(
				widget.NewForm(
					widget.NewFormItem("Name:", at.BoundImageSearchNameEntry),
					widget.NewFormItem("Cols:", at.BoundImageSearchColSplitSlider),
					widget.NewFormItem("Rows:", at.BoundImageSearchRowSplitSlider),

					// widget.NewFormItem("",
					// 	container.NewGridWithColumns(2,
					// 		// colIcon,
					// 		at.BoundImageSearchColSplitSlider,
					// 		// rowIcon,
					// 		// widget.NewLabel("Rows:"),
					// 		at.BoundImageSearchRowSplitSlider,
					// 	),
					// ),
					// widget.NewFormItem("Rows:", at.BoundImageSearchRowSplitEntry),
				),
				nil, nil, nil,
				widget.NewAccordion(
					widget.NewAccordionItem("Search Areas",
						container.NewBorder(
							nil, nil, nil, nil,
							at.ImageSearchSAAccordion,
						),
					),
					widget.NewAccordionItem("Items",
						container.NewBorder(
							nil, nil, nil, nil,
							at.ImageSearchItemsAccordion,
						),
					),
				),
			),
		)
	at.Append(container.NewTabItem("Image", imageSearchSettings))

}

func (at *ActionTabs) constructOcrTab() {
	// 	at.boundOCRSearchAreaSelect = widget.NewSelectWithData(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), binding.NewString())
	// 	at.boundOCRTargetEntry = widget.NewEntryWithData(binding.NewString())

	ocrSettings :=
		widget.NewForm(
			widget.NewFormItem("Text Target:", at.boundOCRTargetEntry),
			widget.NewFormItem("Search Area:", at.OCRSAAccordion),
		)
	at.Append(container.NewTabItem("OCR", ocrSettings))

}

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
