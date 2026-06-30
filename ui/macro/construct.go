package macro

import (
	"image/color"
	"log"

	"Sqyre/internal/assets"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/models/serialize"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

// copiedNodeMap is the clipboard for macro tree copy/paste (nil when empty).
var copiedNodeMap map[string]any

func copyMacroTreeSelection(mt *MacroTree) bool {
	if mt == nil || mt.SelectedNode == "" {
		return false
	}
	node := mt.Macro.Root.GetAction(mt.SelectedNode)
	if node == nil || node.GetParent() == nil {
		return false
	}
	m, err := serialize.ActionToMap(node)
	if err != nil {
		return false
	}
	copiedNodeMap = m
	return true
}

func pasteMacroTreeClipboard(mt *MacroTree) bool {
	if mt == nil {
		return false
	}
	if !mt.PasteNode(copiedNodeMap) {
		return false
	}
	if err := repositories.MacroRepo().Set(mt.Macro.Name, mt.Macro); err != nil {
		log.Printf("failed to save macro after paste: %v", err)
		return false
	}
	return true
}

func moveMacroTreeSelection(mt *MacroTree, up bool) {
	if mt == nil || mt.SelectedNode == "" {
		return
	}
	mt.moveNode(mt.SelectedNode, up)
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
		st.UnselectAll()
		st.SelectedNode = ""
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
	expandAllBtn.SetToolTip("expand all branches")
	collapseAllBtn.SetToolTip("collapse all branches")
	playMacroButton.SetToolTip("start macro execution")
	syncHistoryButtons()

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
					services.MacroActiveIndicator(),
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

	globaldelaytt := ttwidget.NewIcon(theme.HistoryIcon())
	globaldelaytt.SetToolTip("delay between actions (ms)")
	bottomLeftContent := container.NewHBox(globaldelaytt, mui.MTabs.BoundGlobalDelayEntry, mousePosition)
	bottomLeft := wrapFrame(container.NewPadded(bottomLeftContent))
	bottomFiller := canvas.NewRectangle(color.Transparent)
	bottomRightContent := container.NewHBox(
		widget.NewLabel("Hotkey:"),
		mui.MTabs.MacroHotkeyLabel,
		widget.NewLabel("Trigger:"),
		mui.MTabs.HotkeyTriggerRadio,
		mui.MTabs.MacroHotkeyRecordBtn,
	)
	bottomRight := wrapFrame(container.NewPadded(bottomRightContent))
	mui.MacroToolbars.BottomToolbar = container.NewBorder(nil, nil, bottomLeft, bottomRight, bottomFiller)

	return container.NewBorder(
		mui.MacroToolbars.TopToolbar,
		mui.MacroToolbars.BottomToolbar,
		nil,
		nil,
		mui.MTabs,
	)
}
