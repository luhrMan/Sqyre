package macro

import (
	"Sqyre/internal/models"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// MacroTabContent holds the action tree and variables panel for one macro tab.
type MacroTabContent struct {
	widget.BaseWidget
	Tree           *MacroTree
	VariablesPanel *VariablesPanel
	innerTabs      *container.AppTabs
}

// NewMacroTabContent builds the Actions / Variables sub-tabs for a macro.
func NewMacroTabContent(m *models.Macro) *MacroTabContent {
	tree := NewMacroTree(m)
	content := &MacroTabContent{Tree: tree}
	panel := newVariablesPanel(m, func() {
		content.RefreshVariablesPanel()
	})
	content.VariablesPanel = panel
	content.innerTabs = container.NewAppTabs(
		container.NewTabItem("Actions", tree),
		container.NewTabItem("Variables", variablesPanelChrome(panel, m)),
	)
	content.ExtendBaseWidget(content)
	return content
}

func (c *MacroTabContent) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(c.innerTabs)
}

// RefreshVariablesPanel reloads the variables list (call after action edits).
func (c *MacroTabContent) RefreshVariablesPanel() {
	if c != nil && c.VariablesPanel != nil {
		c.VariablesPanel.RefreshDefs()
	}
}

// macroTabContentFrom extracts MacroTabContent from tab item content.
func macroTabContentFrom(obj fyne.CanvasObject) *MacroTabContent {
	if c, ok := obj.(*MacroTabContent); ok {
		return c
	}
	return nil
}
