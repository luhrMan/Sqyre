package macro

import (
	"Sqyre/internal/models"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// LazyMacroTabHost defers building MacroTabContent until the tab is opened.
type LazyMacroTabHost struct {
	widget.BaseWidget
	Macro   *models.Macro
	stack   *fyne.Container
	content *MacroTabContent
}

// NewLazyMacroTabHost returns tab content that builds its tree UI on first access.
func NewLazyMacroTabHost(m *models.Macro) *LazyMacroTabHost {
	h := &LazyMacroTabHost{
		Macro: m,
		stack: container.NewStack(),
	}
	h.ExtendBaseWidget(h)
	return h
}

// EnsureBuilt materializes the full macro tab UI and wires the action tree.
func (h *LazyMacroTabHost) EnsureBuilt() *MacroTabContent {
	if h.content != nil {
		return h.content
	}
	h.content = NewMacroTabContent(h.Macro)
	h.stack.Objects = []fyne.CanvasObject{h.content}
	h.stack.Refresh()
	setMacroTree(h.content.Tree)
	return h.content
}

// Built reports whether the tab content has been constructed.
func (h *LazyMacroTabHost) Built() bool {
	return h.content != nil
}

func (h *LazyMacroTabHost) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(h.stack)
}

// ensureMacroTabContent returns tab content, building lazy hosts when requested.
func ensureMacroTabContent(obj fyne.CanvasObject) *MacroTabContent {
	if c, ok := obj.(*MacroTabContent); ok {
		return c
	}
	if h, ok := obj.(*LazyMacroTabHost); ok {
		return h.EnsureBuilt()
	}
	return nil
}

// macroTabContentFrom returns built tab content, or nil for unbuilt lazy tabs.
func macroTabContentFrom(obj fyne.CanvasObject) *MacroTabContent {
	if c, ok := obj.(*MacroTabContent); ok {
		return c
	}
	if h, ok := obj.(*LazyMacroTabHost); ok {
		return h.content
	}
	return nil
}
