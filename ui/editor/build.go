package editor

import (
	"sync"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
)

var (
	editorBuiltFor *EditorUi
	editorBuildMu  sync.Mutex
)

// IsBuilt reports whether the data editor UI has been constructed.
func IsBuilt() bool {
	editorBuildMu.Lock()
	defer editorBuildMu.Unlock()
	return editorBuiltFor != nil
}

// ResetBuiltForTesting clears the built marker so the next EditorUi is constructed fresh.
func ResetBuiltForTesting() {
	editorBuildMu.Lock()
	defer editorBuildMu.Unlock()
	editorBuiltFor = nil
}

// EnsureBuilt constructs the data editor on first use for each EditorUi instance.
func EnsureBuilt(eu *EditorUi, win fyne.Window) {
	editorBuildMu.Lock()
	defer editorBuildMu.Unlock()
	if eu == editorBuiltFor {
		return
	}

	ConstructEditorTabs(eu, win)
	PrepareToolbarButtons(eu)
	eu.ActionBar = container.NewHBox(layout.NewSpacer(), eu.AddButton, eu.RemoveButton)
	editorScroll := container.NewScroll(eu.EditorTabs)
	eu.CanvasObject = container.NewBorder(
		nil,
		eu.ActionBar,
		nil,
		nil,
		editorScroll,
	)
	eu.RefreshEditorActionBar()
	eu.EditorTabs.OnSelected = func(*container.TabItem) {
		eu.RefreshEditorActionBar()
	}
	editorBuiltFor = eu
}
