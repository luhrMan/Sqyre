package macro

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

func moveMacroTreeSelection(mt *MacroTree, up bool) {
	if mt == nil || mt.SelectedNode == "" {
		return
	}
	mt.moveNode(mt.SelectedNode, up)
}

func canMoveMacroTreeSelection(mt *MacroTree, up bool) bool {
	if mt == nil || mt.SelectedNode == "" || mt.Macro == nil || mt.Macro.Root == nil {
		return false
	}
	node := mt.Macro.Root.GetAction(mt.SelectedNode)
	if node == nil || node.GetParent() == nil {
		return false
	}
	psa := node.GetParent().GetSubActions()
	index := -1
	for i, child := range psa {
		if child == node {
			index = i
			break
		}
	}
	if index < 0 {
		return false
	}
	if up {
		return index > 0
	}
	return index < len(psa)-1
}

func newMacroTreeActionContextMenu(mt *MacroTree) *fyne.Menu {
	unselectItem := fyne.NewMenuItemWithIcon("Unselect", theme.RadioButtonIcon(), func() {
		unselectMacroTreeAction(mt)
	})
	unselectItem.Disabled = mt == nil || mt.SelectedNode == ""

	moveDownItem := fyne.NewMenuItemWithIcon("Move Down", assets.ChevronDownIcon, func() {
		moveMacroTreeSelection(mt, false)
	})
	moveDownItem.Disabled = !canMoveMacroTreeSelection(mt, false)

	moveUpItem := fyne.NewMenuItemWithIcon("Move Up", assets.ChevronUpIcon, func() {
		moveMacroTreeSelection(mt, true)
	})
	moveUpItem.Disabled = !canMoveMacroTreeSelection(mt, true)

	copyItem := fyne.NewMenuItemWithIcon("Copy", theme.ContentCopyIcon(), func() {
		copyMacroTreeSelection(mt)
	})
	copyItem.Disabled = !canCopyMacroTreeSelection(mt)

	pasteItem := fyne.NewMenuItemWithIcon("Paste", theme.ContentPasteIcon(), func() {
		pasteMacroTreeClipboard(mt)
	})
	pasteItem.Disabled = !canPasteMacroTreeClipboard(mt)

	undoItem := fyne.NewMenuItemWithIcon("Undo", theme.ContentUndoIcon(), func() {
		mt.Undo()
	})
	undoItem.Disabled = mt == nil || !mt.CanUndo()

	redoItem := fyne.NewMenuItemWithIcon("Redo", theme.ContentRedoIcon(), func() {
		mt.Redo()
	})
	redoItem.Disabled = mt == nil || !mt.CanRedo()

	expandAllItem := fyne.NewMenuItemWithIcon("Expand All", assets.DoubleDownChevronIcon, func() {
		mt.OpenAllBranches()
	})
	collapseAllItem := fyne.NewMenuItemWithIcon("Collapse All", assets.DoubleUpChevronIcon, func() {
		mt.CloseAllBranches()
	})

	return fyne.NewMenu("",
		unselectItem,
		moveUpItem,
		moveDownItem,
		copyItem,
		pasteItem,
		undoItem,
		redoItem,
		fyne.NewMenuItemSeparator(),
		expandAllItem,
		collapseAllItem,
	)
}

func showMacroTreeActionContextMenu(mt *MacroTree, target fyne.CanvasObject, pe *fyne.PointEvent, uid string) {
	if mt == nil || mt.executing {
		return
	}
	if uid != "" {
		mt.Select(uid)
		mt.SelectedNode = uid
	}
	driver := fyne.CurrentApp().Driver()
	popUpPos := driver.AbsolutePositionForObject(target).Add(pe.Position)
	c := driver.CanvasForObject(target)
	widget.ShowPopUpMenuAtPosition(newMacroTreeActionContextMenu(mt), c, popUpPos)
}

type MacroUi struct {
	MTabs             *MacroTabs
	MacroSelectButton *widget.Button
	MacroToolbars     struct {
		TopToolbar    *fyne.Container
		BottomToolbar *fyne.Container
	}
}

// WrapSqyreFrame matches ui.WrapSqyreFrame; injected to avoid importing package ui.
type WrapSqyreFrameFunc func(inner fyne.CanvasObject) fyne.CanvasObject

// ConstructMacroUi builds the macro toolbar and tab strip. boundLocX/Y labels are bound to mouse position elsewhere.
func ConstructMacroUi(mui *MacroUi, boundLocXLabel, boundLocYLabel *widget.Label, wrapFrame WrapSqyreFrameFunc) *fyne.Container {
	unselectNodeButton := ttwidget.NewButtonWithIcon("", theme.RadioButtonIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		unselectMacroTreeAction(st)
	})

	moveDownNodeButton := ttwidget.NewButtonWithIcon("", assets.ChevronDownIcon, func() {
		moveMacroTreeSelection(mui.MTabs.SelectedTab(), false)
	})
	moveUpNodeButton := ttwidget.NewButtonWithIcon("", assets.ChevronUpIcon, func() {
		moveMacroTreeSelection(mui.MTabs.SelectedTab(), true)
	})
	copyNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentCopyIcon(), func() {
		copyMacroTreeSelection(mui.MTabs.SelectedTab())
	})
	pasteNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentPasteIcon(), func() {
		pasteMacroTreeClipboard(mui.MTabs.SelectedTab())
	})
	undoNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentUndoIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.Undo()
	})
	redoNodeButton := ttwidget.NewButtonWithIcon("", theme.ContentRedoIcon(), func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.Redo()
	})
	syncHistoryButtons := func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			undoNodeButton.Disable()
			redoNodeButton.Disable()
			return
		}
		if st.CanUndo() {
			undoNodeButton.Enable()
		} else {
			undoNodeButton.Disable()
		}
		if st.CanRedo() {
			redoNodeButton.Enable()
		} else {
			redoNodeButton.Disable()
		}
	}
	mui.MTabs.OnHistoryButtonsSync = syncHistoryButtons
	expandAllBtn := ttwidget.NewButtonWithIcon("", assets.DoubleDownChevronIcon, func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.OpenAllBranches()
	})
	collapseAllBtn := ttwidget.NewButtonWithIcon("", assets.DoubleUpChevronIcon, func() {
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		st.CloseAllBranches()
	})
	playMacroButton := ttwidget.NewButtonWithIcon("", theme.MediaPlayIcon(), func() {
		if services.IsMacroRunning() {
			services.RequestMacroStop()
			return
		}
		st := mui.MTabs.SelectedTab()
		if st == nil {
			return
		}
		go services.ExecuteMacroWithLogging(st.Macro)
	})
	moveTabLeftButton := ttwidget.NewButtonWithIcon("", theme.NavigateBackIcon(), func() {
		if mui.MTabs.MoveSelectedTab(-1) {
			SaveOpenMacros()
		}
	})
	moveTabRightButton := ttwidget.NewButtonWithIcon("", theme.NavigateNextIcon(), func() {
		if mui.MTabs.MoveSelectedTab(1) {
			SaveOpenMacros()
		}
	})
	syncTabMoveButtons := func() {
		if mui.MTabs.CanMoveSelectedTab(-1) {
			moveTabLeftButton.Enable()
		} else {
			moveTabLeftButton.Disable()
		}
		if mui.MTabs.CanMoveSelectedTab(1) {
			moveTabRightButton.Enable()
		} else {
			moveTabRightButton.Disable()
		}
	}
	mui.MTabs.OnTabMoveButtonsSync = syncTabMoveButtons
	pauseStatusLabel := widget.NewLabel("")
	pauseStatusLabel.Hide()
	pauseStatusLabel.Wrapping = fyne.TextWrapOff
	pauseStatusLabel.Truncation = fyne.TextTruncateEllipsis
	pauseStatusLabel.Alignment = fyne.TextAlignCenter
	pauseStatusLabel.TextStyle = fyne.TextStyle{Bold: true}
	highlightPump := newHighlightPump(mui)
	services.SetHighlightCallback(highlightPump.handle)
	services.SetMacroPauseStatusCallback(func(status services.MacroPauseStatus) {
		if status.Active {
			text := "Paused"
			if status.ContinueKey != "" {
				text += " — press " + status.ContinueKey + " to continue"
			}
			if status.Message != "" {
				text += " — " + status.Message
			}
			pauseStatusLabel.SetText(text)
			pauseStatusLabel.Show()
		} else {
			pauseStatusLabel.SetText("")
			pauseStatusLabel.Hide()
		}
	})
	services.SetMacroRunningCallback(func(running bool) {
		for _, t := range mui.MTabs.AllTrees() {
			t.SetExecuting(running)
		}
		if running {
			playMacroButton.SetIcon(theme.MediaStopIcon())
			playMacroButton.SetToolTip("stop macro execution")
			startLogPump()
			highlightPump.startTicker()
		} else {
			playMacroButton.SetIcon(theme.MediaPlayIcon())
			playMacroButton.SetToolTip("start macro execution")
			stopLogPump()
			highlightPump.stopTicker()
		}
	})

	unselectNodeButton.SetToolTip("unselect nodes")
	moveDownNodeButton.SetToolTip("move node down (Alt+Down)")
	moveUpNodeButton.SetToolTip("move node up (Alt+Up)")
	copyNodeButton.SetToolTip("copy node (Ctrl+C)")
	pasteNodeButton.SetToolTip("paste node below (Ctrl+V)")
	undoNodeButton.SetToolTip("undo (Ctrl+Z)")
	redoNodeButton.SetToolTip("redo (Ctrl+Y)")
	macroActiveIndicator := widget.NewActivity()
	services.SetMacroIndicatorUI(services.MacroIndicatorUI{
		Show:  macroActiveIndicator.Show,
		Hide:  macroActiveIndicator.Hide,
		Start: macroActiveIndicator.Start,
		Stop:  macroActiveIndicator.Stop,
	})
	expandAllBtn.SetToolTip("expand all branches")
	collapseAllBtn.SetToolTip("collapse all branches")
	playMacroButton.SetToolTip("start macro execution")
	moveTabLeftButton.SetToolTip("move macro tab left")
	moveTabRightButton.SetToolTip("move macro tab right")
	syncHistoryButtons()
	syncTabMoveButtons()

	mui.MacroToolbars.TopToolbar =
		container.NewGridWithColumns(2,
			container.NewBorder(
				nil, nil,
				container.NewHBox(
					unselectNodeButton,
					moveDownNodeButton,
					moveUpNodeButton,
					copyNodeButton,
					pasteNodeButton,
					undoNodeButton,
					redoNodeButton,
					expandAllBtn,
					collapseAllBtn,
				),
				container.NewHBox(
					playMacroButton,
					macroActiveIndicator,
					moveTabLeftButton,
					moveTabRightButton,
					widget.NewLabel("Macro Name:"),
				),
				pauseStatusLabel,
			),
			container.NewBorder(nil, nil, nil,
				mui.MacroSelectButton,
				mui.MTabs.MacroNameEntry,
			),
		)

	mousePosition :=
		container.NewHBox(
			container.NewBorder(nil, nil,
				widget.NewLabel("X: "), nil,
				boundLocXLabel,
			),
			container.NewBorder(nil, nil,
				widget.NewLabel("Y: "), nil,
				boundLocYLabel,
			),
		)

	bottomLeftContent := container.NewHBox(mui.MTabs.MacroDelayBtn, mousePosition)
	bottomLeft := wrapFrame(container.NewPadded(bottomLeftContent))
	bottomCenterContent := container.NewBorder(nil, nil,
		widget.NewLabel("Tags:"),
		mui.MTabs.MacroTagSubmitBtn,
		container.NewBorder(nil, nil,
			mui.MTabs.MacroTagsBtn,
			nil,
			mui.MTabs.MacroTagEntry,
		),
	)
	bottomCenter := wrapFrame(container.NewPadded(bottomCenterContent))
	bottomRightContent := container.NewHBox(
		widget.NewLabel("Hotkey:"),
		mui.MTabs.MacroHotkeyLabel,
		widget.NewLabel("Trigger:"),
		mui.MTabs.HotkeyTriggerRadio,
		mui.MTabs.MacroHotkeyRecordBtn,
		mui.MTabs.MacroHotkeyClearBtn,
	)
	bottomRight := wrapFrame(container.NewPadded(bottomRightContent))
	mui.MacroToolbars.BottomToolbar = container.NewBorder(nil, nil, bottomLeft, bottomRight, bottomCenter)

	return container.NewBorder(
		mui.MacroToolbars.TopToolbar,
		mui.MacroToolbars.BottomToolbar,
		nil,
		nil,
		mui.MTabs,
	)
}
