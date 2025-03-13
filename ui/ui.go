package ui

import (
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	widget "fyne.io/fyne/v2/widget"
	xwidget "fyne.io/x/fyne/widget"
)

var savedMacrosPath = "./internal/data/resources/saved-macros/"

type Ui struct {
	win fyne.Window

	mm  map[string]*MacroTree
	sel *xwidget.CompletionEntry

	dt *container.DocTabs
	st *settingsTabs
}

func (u *Ui) SetWindow(w fyne.Window)            { u.win = w }
func (u *Ui) SetMacros(mm map[string]*MacroTree) { u.mm = mm }
func (u *Ui) CreateSettingsTabs()                { u.st = &settingsTabs{tabs: &container.AppTabs{}} }
func (u *Ui) createDocTabs()                     { u.dt = container.NewDocTabs() }

type settingsTabs struct {
	tabs                  *container.AppTabs
	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry
	waitTab
	moveTab
	keyTab
	loopTab
	imageSearchTab
	ocrTab
}

type waitTab struct {
	boundTime binding.Int

	boundTimeSlider *widget.Slider
	boundTimeEntry  *widget.Entry
}

type moveTab struct {
	boundMoveX binding.Int
	boundMoveY binding.Int
	boundSpot  binding.String

	boundMoveXSlider *widget.Slider
	boundMoveYSlider *widget.Slider
	boundMoveXEntry  *widget.Entry
	boundMoveYEntry  *widget.Entry
	boundSpotSelect  *widget.Select
}

type keyTab struct {
	boundButton binding.Bool
	boundKey    binding.String
	boundState  binding.Bool

	boundButtonToggle *custom_widgets.Toggle
	boundKeySelect    *widget.Select
	boundStateToggle  *custom_widgets.Toggle
}

type loopTab struct {
	boundLoopName binding.String
	boundCount    binding.Int

	boundLoopNameEntry *widget.Entry
	boundCountSlider   *widget.Slider
	boundCountLabel    *widget.Label
}

type imageSearchTab struct {
	boundImageSearchName binding.String
	boundImageSearchArea binding.String
	boundXSplit          binding.Int
	boundYSplit          binding.Int

	boundImageSearchNameEntry  *widget.Entry
	boundImageSearchAreaSelect *widget.Select
	boundXSplitSlider          *widget.Slider
	boundXSplitEntry           *widget.Entry
}

type ocrTab struct {
	boundOCRTarget    binding.String
	boundOCRSearchBox binding.String

	boundOCRTargetEntry     *widget.Entry
	boundOCRSearchBoxSelect *widget.Select
}
