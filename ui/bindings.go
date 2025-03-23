package ui

import (
	"Squire/internal/actions"
	"Squire/internal/data"
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

func (u *Ui) bindVariables() {
	// ct.boundMacroName = binding.BindString(&macroName)
	u.st.boundGlobalDelay = binding.BindInt(&globalDelay)
	u.st.boundGlobalDelay.AddListener(binding.NewDataListener(func() { robotgo.MouseSleep = globalDelay; robotgo.KeySleep = globalDelay }))
	u.st.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundGlobalDelay))
	u.st.boundGlobalDelay.AddListener(binding.NewDataListener(func() {
		u.selectedMacroTab().Macro.GlobalDelay = globalDelay
	}))
	u.st.boundTime = binding.BindInt(&time)
	u.st.boundTimeEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundTime))
	u.st.boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, binding.IntToFloat(u.st.boundTime))
	u.st.boundTime.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Wait); ok {
			n.Time = time
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundMoveX = binding.BindInt(&moveX)
	u.st.boundMoveY = binding.BindInt(&moveY)
	u.st.boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(data.MonitorWidth), binding.IntToFloat(u.st.boundMoveX))
	u.st.boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(data.MonitorHeight), binding.IntToFloat(u.st.boundMoveY))
	u.st.boundMoveXEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundMoveX))
	u.st.boundMoveYEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundMoveY))
	u.st.boundSpot = binding.BindString(&spot)
	u.st.boundSpotSelect = widget.NewSelect(*data.GetPointMapKeys(data.JsonPointMap()), func(s string) {
		u.st.boundSpot.Set(s)
		u.st.boundMoveX.Set(data.GetPoint(s).X)
		u.st.boundMoveY.Set(data.GetPoint(s).Y)
	})
	u.st.boundMoveX.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Move); ok {
			n.X = moveX
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundMoveY.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Move); ok {
			n.Y = moveY
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundButton = binding.BindBool(&button)
	u.st.boundButtonToggle = custom_widgets.NewToggleWithData(u.st.boundButton)
	u.st.boundButton.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Click); ok {
			n.Button = actions.LeftOrRight(button)
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundKey = binding.BindString(&key)
	u.st.boundKeySelect = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { u.st.boundKey.Set(s) })
	u.st.boundState = binding.BindBool(&state)
	u.st.boundStateToggle = custom_widgets.NewToggleWithData(u.st.boundState)
	u.st.boundKey.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Key); ok {
			n.Key = key
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundState.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Key); ok {
			n.State = actions.UpOrDown(state)
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundLoopName = binding.BindString(&loopName)
	u.st.boundCount = binding.BindInt(&count)
	u.st.boundLoopNameEntry = widget.NewEntryWithData(u.st.boundLoopName)
	u.st.boundCountSlider = widget.NewSliderWithData(1, 10, binding.IntToFloat(u.st.boundCount))
	u.st.boundCountLabel = widget.NewLabelWithData(binding.IntToString(u.st.boundCount))
	u.st.boundLoopName.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Loop); ok {
			n.Name = loopName
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundCount.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.Loop); ok {
			n.Count = count
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundImageSearchName = binding.BindString(&imageSearchName)
	u.st.boundImageSearchArea = binding.BindString(&searchArea)
	u.st.boundXSplit = binding.BindInt(&xSplit)
	u.st.boundYSplit = binding.BindInt(&ySplit)
	u.st.boundImageSearchNameEntry = widget.NewEntryWithData(u.st.boundImageSearchName)
	u.st.boundImageSearchAreaSelect = widget.NewSelect(*data.GetSearchAreaMapKeys(*data.GetSearchAreaMap()), func(s string) { u.st.boundImageSearchArea.Set(s) })

	u.st.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(u.st.boundXSplit))
	u.st.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundXSplit))
	u.st.boundImageSearchName.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.Name = imageSearchName
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundImageSearchArea.AddListener(binding.NewDataListener(func() {
		if n, ok := u.selectedMacroTab().Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.SearchArea = *data.GetSearchArea(searchArea)
			u.selectedMacroTab().Tree.Refresh()
		}
	}))
	u.st.boundOCRSearchBox = binding.BindString(&ocrSearchBox)
	u.st.boundOCRTarget = binding.BindString(&ocrTarget)
	u.st.boundOCRSearchBoxSelect = widget.NewSelect(*data.GetSearchAreaMapKeys(*data.GetSearchAreaMap()), func(s string) { u.st.boundOCRSearchBox.Set(s) })
	u.st.boundOCRTargetEntry = widget.NewEntryWithData(u.st.boundOCRTarget)

}
