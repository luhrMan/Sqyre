package editor

import (
	"sync"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
)

var (
	editorBuilt   bool
	editorBuildMu sync.Mutex
)

// IsBuilt reports whether the data editor UI has been constructed.
func IsBuilt() bool {
	editorBuildMu.Lock()
	defer editorBuildMu.Unlock()
	return editorBuilt
}

// EnsureBuilt constructs the data editor on first use.
func EnsureBuilt(eu *EditorUi, win fyne.Window) {
	editorBuildMu.Lock()
	defer editorBuildMu.Unlock()
	if editorBuilt {
		return
	}

	ConstructEditorTabs(eu, win)
	PrepareToolbarButtons(eu)
	eu.ActionBar = container.NewHBox(layout.NewSpacer(), eu.AddButton, eu.RemoveButton)
	eu.CanvasObject = container.NewBorder(
		nil,
		eu.ActionBar,
		nil,
		nil,
		eu.EditorTabs,
	)
	eu.RefreshEditorActionBar()
	eu.EditorTabs.OnSelected = func(*container.TabItem) {
		eu.RefreshEditorActionBar()
	}
	editorBuilt = true
}
