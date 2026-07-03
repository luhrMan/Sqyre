package macro

import (
	"strings"

	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// maxLogLines bounds how many recent log lines are kept in the on-screen Entry.
// The full log is always available via services.GetMacroLogBuffer (and sqyre.log).
// Bounding keeps each Entry.SetText cheap and constant-cost regardless of run
// length, avoiding the quadratic slowdown of appending to an ever-growing entry.
const maxLogLines = 400

// MacroTabContent holds the action tree, variables panel, and execution log for one macro tab.
type MacroTabContent struct {
	widget.BaseWidget
	Macro          *models.Macro
	Tree           *MacroTree
	VariablesPanel *VariablesPanel
	innerTabs      *container.AppTabs
	logTab         *container.TabItem
	liveVarsTab    *container.TabItem

	logEntry        *widget.Entry
	logScroll       *container.Scroll
	logLines        []string
	logRenderedLines int
	refreshLiveVars func()
	liveVarsEmpty   *widget.Label
	liveVarsScroll  *container.Scroll
}

// NewMacroTabContent builds the Actions / Variables / Live variables / Log sub-tabs for a macro.
func NewMacroTabContent(m *models.Macro) *MacroTabContent {
	tree := NewMacroTree(m)
	content := &MacroTabContent{Macro: m, Tree: tree}
	panel := newVariablesPanel(m, func() {
		content.RefreshVariablesPanel()
	})
	content.VariablesPanel = panel

	logEntry := widget.NewMultiLineEntry()
	logEntry.Disable()
	logEntry.Wrapping = fyne.TextWrapOff
	logScroll := container.NewScroll(logEntry)
	copyBtn := widget.NewButtonWithIcon("Copy", theme.ContentCopyIcon(), func() {
		full := services.GetMacroLogBuffer()
		if full == "" {
			full = logEntry.Text
		}
		_ = screen.WriteClipboard(full)
	})
	clearBtn := widget.NewButtonWithIcon("Clear", theme.DeleteIcon(), func() {
		logEntry.SetText("")
		content.logRenderedLines = 0
		content.logLines = content.logLines[:0]
	})
	logBar := container.NewHBox(layout.NewSpacer(), clearBtn, copyBtn)
	logPane := container.NewBorder(nil, logBar, nil, nil, logScroll)
	content.logEntry = logEntry
	content.logScroll = logScroll

	varsList, refreshVars := buildRuntimeVariablesView()
	varsScroll := container.NewScroll(varsList)
	emptyVarsLabel := widget.NewLabel("No variables set yet.")
	emptyVarsLabel.Alignment = fyne.TextAlignCenter
	varsScroll.Hide()
	varsPane := container.NewStack(emptyVarsLabel, varsScroll)
	content.refreshLiveVars = refreshVars
	content.liveVarsEmpty = emptyVarsLabel
	content.liveVarsScroll = varsScroll

	logTab := container.NewTabItem("Log", logPane)
	content.logTab = logTab
	liveVarsTab := container.NewTabItem("Live variables", varsPane)
	content.liveVarsTab = liveVarsTab
	content.innerTabs = container.NewAppTabs(
		container.NewTabItem("Actions", buildActionsPane(tree)),
		container.NewTabItem("Variables", variablesPanelChrome(panel, m)),
		liveVarsTab,
		logTab,
	)
	content.innerTabs.OnSelected = func(*container.TabItem) {
		if content.logTabVisible() {
			content.syncLogEntry()
		}
	}
	content.ExtendBaseWidget(content)
	return content
}

func (c *MacroTabContent) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(c.innerTabs)
}

// buildActionsPane stacks the macro tree with a lightweight drop-preview overlay.
func buildActionsPane(tree *MacroTree) fyne.CanvasObject {
	dropGhost, dropGhostInset, dropGhostRow := newDropGhostShell()
	overlay := container.NewWithoutLayout(dropGhost)
	tree.attachDropOverlay(overlay, dropGhost, dropGhostInset, dropGhostRow)
	return container.NewStack(tree, overlay)
}

// GoToAction switches to the Actions sub-tab and navigates to the action with uid.
func (c *MacroTabContent) GoToAction(uid string) {
	if c == nil || c.Tree == nil || uid == "" {
		return
	}
	if c.innerTabs != nil {
		c.innerTabs.SelectIndex(0)
	}
	c.Tree.GoToAction(uid)
}

// RefreshVariablesPanel reloads the variables list (call after action edits).
func (c *MacroTabContent) RefreshVariablesPanel() {
	if c != nil && c.VariablesPanel != nil {
		c.VariablesPanel.RefreshDefs()
	}
}

// updateLiveVars refreshes the live variables list and toggles the empty-state label.
func (c *MacroTabContent) updateLiveVars() {
	if c.refreshLiveVars != nil {
		c.refreshLiveVars()
	}
	if len(services.GetRuntimeVariables()) == 0 {
		c.liveVarsEmpty.Show()
		c.liveVarsScroll.Hide()
	} else {
		c.liveVarsEmpty.Hide()
		c.liveVarsScroll.Show()
	}
}

// BindLog starts log capture for the running macro and makes this tab the target
// of the log pump (which drains buffered lines on a fixed-rate timer). Called on
// the UI thread when execution starts.
func (c *MacroTabContent) BindLog(macroName string) {
	c.logLines = c.logLines[:0]
	c.logRenderedLines = 0
	c.logEntry.SetText("")
	c.logScroll.ScrollToBottom()
	c.updateLiveVars()

	services.StartMacroLogCapture(macroName)
	setActiveLogContent(c)
}

func (c *MacroTabContent) logTabVisible() bool {
	if c == nil || c.innerTabs == nil || c.logTab == nil {
		return false
	}
	return c.innerTabs.Selected() == c.logTab
}

func (c *MacroTabContent) liveVarsTabVisible() bool {
	if c == nil || c.innerTabs == nil || c.liveVarsTab == nil {
		return false
	}
	return c.innerTabs.Selected() == c.liveVarsTab
}

func (c *MacroTabContent) syncLogEntry() {
	if c == nil || c.logEntry == nil {
		return
	}
	c.logEntry.SetText(strings.Join(c.logLines, "\n"))
	c.logRenderedLines = len(c.logLines)
	c.logScroll.ScrollToBottom()
}

// appendDrainedLog appends a batch of drained log lines, trims to the cap, and
// updates the Entry incrementally when the Log tab is visible.
func (c *MacroTabContent) appendDrainedLog(lines []string) {
	if len(lines) == 0 {
		return
	}
	trimmed := len(c.logLines)+len(lines) > maxLogLines
	c.logLines = append(c.logLines, lines...)
	if len(c.logLines) > maxLogLines {
		c.logLines = append([]string(nil), c.logLines[len(c.logLines)-maxLogLines:]...)
	}
	if !c.logTabVisible() {
		c.logRenderedLines = 0
		return
	}
	if trimmed || c.logRenderedLines == 0 {
		c.syncLogEntry()
		return
	}
	c.logEntry.SetText(c.logEntry.Text + "\n" + strings.Join(lines, "\n"))
	c.logRenderedLines += len(lines)
	c.logScroll.ScrollToBottom()
}
