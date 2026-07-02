// Package completionentry provides a CompletionEntry widget that guards against
// nil pointer dereferences when the widget is not on a canvas (e.g. after
// navigation back). Based on fyne.io/x/fyne/widget.CompletionEntry with
// defensive nil checks for CanvasForObject and navigableList.
package completionentry

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// CompletionEntry is an Entry with options displayed in a PopUpMenu.
type CompletionEntry struct {
	widget.Entry
	popupMenu     *widget.PopUp
	navigableList *navigableList
	Options       []string
	pause         bool
	itemHeight    float32
	visible       bool

	CustomCreate func() fyne.CanvasObject
	CustomUpdate func(id widget.ListItemID, object fyne.CanvasObject)
}

// NewCompletionEntry creates a new CompletionEntry which creates a popup menu that responds to keystrokes to navigate through the items without losing the editing ability of the text input.
func NewCompletionEntry(options []string) *CompletionEntry {
	c := &CompletionEntry{Options: options}
	c.ExtendBaseWidget(c)
	return c
}

// HideCompletion hides the completion menu.
func (c *CompletionEntry) HideCompletion() {
	if c.popupMenu != nil {
		c.popupMenu.Hide()
	}
	if c.visible {
		c.visible = false
		registerCompletionHidden()
	}
}

// Move changes the relative position of the select entry.
//
// Implements: fyne.Widget
func (c *CompletionEntry) Move(pos fyne.Position) {
	c.Entry.Move(pos)
	if c.popupMenu != nil && c.onCanvas() {
		c.popupMenu.Resize(c.maxSize())
		c.popupMenu.Move(c.popUpPos())
	}
}

// Refresh the list to update the options to display.
func (c *CompletionEntry) Refresh() {
	c.Entry.Refresh()
	if c.navigableList != nil {
		c.navigableList.SetOptions(c.Options)
	}
}

// Resize sets a new size for a widget.
// Note this should not be used if the widget is being managed by a Layout within a Container.
func (c *CompletionEntry) Resize(size fyne.Size) {
	c.Entry.Resize(size)
	if c.popupMenu != nil && c.onCanvas() {
		c.popupMenu.Resize(c.maxSize())
	}
}

// SetOptions set the completion list with itemList and update the view.
func (c *CompletionEntry) SetOptions(itemList []string) {
	c.Options = itemList
	c.Refresh()
}

// ShowCompletion displays the completion menu
func (c *CompletionEntry) ShowCompletion() {
	if c.pause {
		return
	}
	if len(c.Options) == 0 {
		c.HideCompletion()
		return
	}

	if c.navigableList == nil {
		c.navigableList = newNavigableList(c.Options, &c.Entry, c.setTextFromMenu, c.HideCompletion,
			c.CustomCreate, c.CustomUpdate)
	} else {
		c.navigableList.UnselectAll()
		c.navigableList.selected = -1
	}
	holder := fyne.CurrentApp().Driver().CanvasForObject(c)
	if holder == nil {
		return
	}

	if c.popupMenu == nil {
		c.popupMenu = widget.NewPopUp(c.navigableList, holder)
	}
	c.popupMenu.Resize(c.maxSize())
	c.popupMenu.ShowAtPosition(c.popUpPos())
	holder.Focus(c.navigableList)
	if !c.visible {
		registerCompletionShown()
	}
	c.visible = true
}

// onCanvas returns true if the widget is currently attached to a canvas (e.g. visible).
func (c *CompletionEntry) onCanvas() bool {
	return fyne.CurrentApp().Driver().CanvasForObject(c) != nil
}

func (c *CompletionEntry) popupGeometry() popupGeometry {
	measureItemHeight := func() float32 {
		if c.navigableList == nil {
			return 0
		}
		c.itemHeight = c.navigableList.CreateItem().MinSize().Height
		return c.itemHeight
	}
	return popupGeometryFor(c, len(c.Options), c.itemHeight, measureItemHeight)
}

func (c *CompletionEntry) maxSize() fyne.Size {
	return c.popupGeometry().size
}

func (c *CompletionEntry) popUpPos() fyne.Position {
	return c.popupGeometry().pos
}

// Prevent the menu to open when the user validate value from the menu.
func (c *CompletionEntry) setTextFromMenu(s string) {
	c.pause = true
	c.Entry.SetText(s)
	c.Entry.CursorColumn = len([]rune(s))
	c.Entry.Refresh()
	c.pause = false
	c.HideCompletion()
}

type navigableList struct {
	widget.List
	entry           *widget.Entry
	selected        int
	setTextFromMenu func(string)
	hide            func()
	navigating      bool
	items           []string
	labels          []string

	customCreate func() fyne.CanvasObject
	customUpdate func(id widget.ListItemID, object fyne.CanvasObject)
}

func newNavigableList(items []string, entry *widget.Entry, setTextFromMenu func(string), hide func(),
	create func() fyne.CanvasObject, update func(id widget.ListItemID, object fyne.CanvasObject)) *navigableList {
	n := &navigableList{
		entry:           entry,
		selected:        -1,
		setTextFromMenu: setTextFromMenu,
		hide:            hide,
		items:           items,
		customCreate:    create,
		customUpdate:    update,
	}

	n.List = widget.List{
		Length: func() int {
			return len(n.items)
		},
		CreateItem: func() fyne.CanvasObject {
			if fn := n.customCreate; fn != nil {
				return fn()
			}
			return widget.NewLabel("")
		},
		UpdateItem: func(i widget.ListItemID, o fyne.CanvasObject) {
			if fn := n.customUpdate; fn != nil {
				fn(i, o)
				return
			}
			text := n.items[i]
			if i < len(n.labels) && n.labels[i] != "" {
				text = n.labels[i]
			}
			o.(*widget.Label).SetText(text)
		},
		OnSelected: func(id widget.ListItemID) {
			if !n.navigating && id > -1 {
				setTextFromMenu(n.items[id])
			}
			n.navigating = false
		},
	}
	n.ExtendBaseWidget(n)
	return n
}

// Implements: fyne.Focusable
func (n *navigableList) FocusGained() {
}

// Implements: fyne.Focusable
func (n *navigableList) FocusLost() {
}

func (n *navigableList) SetOptions(items []string) {
	n.Unselect(n.selected)
	n.items = items
	if len(n.labels) != len(items) {
		n.labels = nil
	}
	n.Refresh()
	n.selected = -1
}

func (n *navigableList) SetLabels(labels []string) {
	n.labels = labels
	n.Refresh()
}

func (n *navigableList) TypedKey(event *fyne.KeyEvent) {
	switch event.Name {
	case fyne.KeyDown:
		if n.selected < len(n.items)-1 {
			n.selected++
		} else {
			n.selected = 0
		}
		n.navigating = true
		n.Select(n.selected)

	case fyne.KeyUp:
		if n.selected > 0 {
			n.selected--
		} else {
			n.selected = len(n.items) - 1
		}
		n.navigating = true
		n.Select(n.selected)
	case fyne.KeyReturn, fyne.KeyEnter:
		suppressActionDialogEnter()
		if n.selected == -1 { // so the user want to submit the entry
			n.hide()
			n.entry.TypedKey(event)
		} else {
			n.navigating = false
			n.OnSelected(n.selected)
		}
	case fyne.KeyEscape:
		n.hide()
	default:
		n.entry.TypedKey(event)

	}
}

func (n *navigableList) TypedRune(r rune) {
	n.entry.TypedRune(r)
}
