package ui

import (
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
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
	BoundImageSearchColSplitLabel  *ttwidget.Label
	BoundImageSearchRowSplitLabel  *ttwidget.Label
	ImageSearchSAAccordion         *widget.Accordion
	ImageSearchItemsAccordion      *widget.Accordion

	BoundOcrNameEntry   *widget.Entry
	BoundOcrTargetEntry *widget.Entry
	OcrSAAccordion      *widget.Accordion
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
		BoundImageSearchColSplitLabel:  ttwidget.NewLabel(""),
		BoundImageSearchRowSplitLabel:  ttwidget.NewLabel(""),

		ImageSearchSAAccordion:    widget.NewAccordion(),
		ImageSearchItemsAccordion: widget.NewAccordion(),

		BoundOcrNameEntry:   new(widget.Entry),
		BoundOcrTargetEntry: new(widget.Entry),
		OcrSAAccordion:      widget.NewAccordion(),
	}
}

func (u *Ui) constructActionTabs() *ActionTabs {
	at := u.ActionTabs
	waitSettings :=
		widget.NewForm(
			widget.NewFormItem("ms", container.NewGridWithColumns(2,
				at.BoundTimeEntry, at.BoundTimeSlider,
			)),
		)
	moveSettings :=
		container.NewBorder(
			nil, nil, nil, nil,
			at.PointsAccordion,
		)
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
	loopSettings :=
		widget.NewForm(
			widget.NewFormItem("Name:", at.BoundLoopNameEntry),
			widget.NewFormItem("Loops:", container.NewBorder(
				nil, nil, at.BoundCountLabel, nil, at.BoundCountSlider,
			)),
		)
	// colIcon := ttwidget.NewIcon(theme.NewDisabledResource(theme.MoreVerticalIcon()))
	// colIcon.SetToolTip("columns split")
	// rowIcon := ttwidget.NewIcon(theme.NewDisabledResource(theme.MoreHorizontalIcon()))
	// rowIcon.SetToolTip("rows split")
	// at.BoundImageSearchColSplitSlider.OnChanged = func(f float64) {
	// 	at.BoundImageSearchColSplitSlider.SetToolTip(strconv.FormatFloat(f, 'f', -1, 64))
	// }
	colIcon := ttwidget.NewIcon(theme.NewPrimaryThemedResource(theme.MoreVerticalIcon()))
	rowIcon := ttwidget.NewIcon(theme.NewErrorThemedResource(theme.MoreHorizontalIcon()))
	colIcon.SetToolTip("column search split")
	rowIcon.SetToolTip("row search split")
	at.BoundImageSearchColSplitSlider.SetToolTip("column search split")
	at.BoundImageSearchRowSplitSlider.SetToolTip("row search split")
	imageSearchSettings :=
		container.NewScroll(
			container.NewBorder(
				container.NewVBox(
					container.NewBorder(nil, nil, widget.NewLabel("Name:"), nil, at.BoundImageSearchNameEntry),
					container.NewBorder(
						nil, nil,
						container.NewBorder(nil, nil, colIcon, at.BoundImageSearchColSplitLabel), nil,
						at.BoundImageSearchColSplitSlider,
					),
					container.NewBorder(
						nil, nil,
						container.NewBorder(nil, nil, rowIcon, at.BoundImageSearchRowSplitLabel), nil,
						at.BoundImageSearchRowSplitSlider,
					),
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

	ocrSettings :=
		container.NewBorder(
			container.NewGridWithRows(2,
				container.NewBorder(
					nil, nil,
					widget.NewLabel("Name:"), nil,
					at.BoundOcrNameEntry,
				),
				container.NewBorder(
					nil, nil,
					widget.NewLabel("Text Target:"), nil,
					at.BoundOcrTargetEntry,
				),
			),
			nil, nil, nil,
			at.OcrSAAccordion,
		)

	at.Append(container.NewTabItem("Wait", waitSettings))
	at.Append(container.NewTabItem("Move", moveSettings))
	at.Append(container.NewTabItem("Click", clickSettings))
	at.Append(container.NewTabItem("Key", keySettings))
	at.Append(container.NewTabItem("Loop", loopSettings))
	at.Append(container.NewTabItem("Image", imageSearchSettings))
	at.Append(container.NewTabItem("OCR", ocrSettings))
	return u.ActionTabs
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
