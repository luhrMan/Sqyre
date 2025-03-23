package ui

import (
	"Squire/internal"
	"Squire/internal/data"
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	widget "fyne.io/fyne/v2/widget"
	xwidget "fyne.io/x/fyne/widget"
)

var savedMacrosPath = data.ResourcePath + "saved-macros/"

type Ui struct {
	win fyne.Window

	mtm map[string]*MacroTree
	sel *xwidget.CompletionEntry

	dt *container.DocTabs
	st *settingsTabs
}

func InitializeUi(w fyne.Window) *Ui {
	return &Ui{
		win: w,
		mtm: map[string]*MacroTree{},
	}
}

func (u *Ui) SetWindow(w fyne.Window)                { u.win = w }
func (u *Ui) AddMacroTree(key string, mt *MacroTree) { u.mtm[key] = mt }
func (u *Ui) CreateSettingsTabs()                    { u.st = &settingsTabs{tabs: &container.AppTabs{}} }
func (u *Ui) CreateDocTabs() {
	u.dt = container.NewDocTabs()
	for _, m := range internal.GetPrograms().GetProgram(data.DarkAndDarker).Macros {
		u.addMacroDocTab(m)
	}
	u.dt.SelectIndex(0)
}

type settingsTabs struct {
	tabs                  *container.AppTabs
	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry
	waitTab
	moveTab
	clickTab
	keyTab
	loopTab
	imageSearchTab
	ocrTab
}

// settingsTabs indexes
const (
	waittab = iota
	movetab
	clicktab
	keytab
	looptab
	imagesearchtab
	ocrtab
)

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

type clickTab struct {
	boundButton binding.Bool

	boundButtonToggle *custom_widgets.Toggle
}

type keyTab struct {
	boundKey   binding.String
	boundState binding.Bool

	boundKeySelect   *widget.Select
	boundStateToggle *custom_widgets.Toggle
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
