package ui

import (
	"fyne.io/fyne/v2"
)

type overlayKind int

const (
	overlayNone overlayKind = iota
	overlayEditor
	overlaySettings
)

// wireNavigation hooks back/forward so overlay tracking stays in sync with the stack.
func (u *Ui) wireNavigation() {
	nav := u.MainUi.Navigation
	nav.OnBack = func() {
		if nav.Back() != nil {
			u.MainUi.overlayKind = overlayNone
		}
	}
	nav.OnForward = func() {
		if o := nav.Forward(); o != nil {
			u.MainUi.overlayKind = u.overlayKindFor(o)
		}
	}
}

func (u *Ui) overlayKindFor(obj fyne.CanvasObject) overlayKind {
	switch obj {
	case u.EditorUi.CanvasObject:
		return overlayEditor
	case u.SettingsUi.CanvasObject:
		return overlaySettings
	default:
		return overlayNone
	}
}

// showOverlay navigates to a secondary screen without duplicating CanvasObjects on the stack.
// Fyne's Navigation.PushWithTitle must not be called with an object that is already on the stack.
func (u *Ui) showOverlay(obj fyne.CanvasObject, title string, kind overlayKind) {
	if obj == nil {
		return
	}
	nav := u.MainUi.Navigation
	if u.MainUi.overlayKind == kind {
		return
	}
	if u.MainUi.overlayKind != overlayNone {
		nav.Back()
		u.MainUi.overlayKind = overlayNone
	}
	if next := nav.Forward(); next == obj {
		u.MainUi.overlayKind = kind
		nav.SetCurrentTitle(title)
		return
	} else if next != nil {
		nav.Back()
	}
	nav.PushWithTitle(obj, title)
	u.MainUi.overlayKind = kind
	if kind == overlayEditor {
		clampWindowToScreen(u.Window)
	}
}
