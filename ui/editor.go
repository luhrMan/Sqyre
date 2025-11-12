package ui

import (
	"Squire/internal/services"
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

type EditorUi struct {
	fyne.CanvasObject
	NavButton       *widget.Button
	AddButton       *widget.Button
	RemoveButton    *widget.Button
	ProgramSelector *widget.SelectEntry
	EditorTabs      struct {
		*container.AppTabs
		ProgramsTab    *EditorTab
		ItemsTab       *EditorTab
		PointsTab      *EditorTab
		SearchAreasTab *EditorTab
	}
}
type EditorTab struct {
	*container.TabItem
	Split *container.Split
	Left  *fyne.Container
	Right *fyne.Container

	Widgets      map[string]fyne.Widget
	SelectedItem any
}

func NewEditorTab(name string, left, right *fyne.Container) *container.TabItem {
	split := container.NewHSplit(left, right)
	return container.NewTabItem(name, split)
}

func (u *Ui) constructEditorTabs() {
	var (
		name = "Name"
		x    = "X"
		y    = "Y"
		x1   = "RightX"
		y1   = "TopY"
		x2   = "LeftX"
		y2   = "BottomY"
		cols = "Cols"
		rows = "Rows"
		tags = "Tags"
		sm   = "StackMax"
		// m    = "Merchant"
		form  = "Form"
		acc   = "Accordion"
		plist = "list"

		et    = ui.EditorTabs
		protw = et.ProgramsTab.Widgets
		itw   = et.ItemsTab.Widgets
		ptw   = et.PointsTab.Widgets
		satw  = et.SearchAreasTab.Widgets
	)

	protw[name] = new(widget.Entry)
	protw[plist] = new(widget.List)
	protw["searchbar"] = new(widget.Entry)
	protw[form] = widget.NewForm(
		widget.NewFormItem(name, protw[name]),
	)
	protw[form].(*widget.Form).SubmitText = "Update"

	et.ProgramsTab.TabItem = NewEditorTab(
		"Programs",
		container.NewBorder(protw["searchbar"], nil, nil, nil, protw[plist]),
		container.NewBorder(nil, nil, nil, nil, protw[form]),
	)

	//===========================================================================================================ITEMS
	itw[acc] = widget.NewAccordion()
	itw[name] = new(widget.Entry)
	itw[cols] = new(widget.Entry)
	itw[rows] = new(widget.Entry)
	itw[tags] = widget.NewCard("test", "", nil)
	itw[sm] = new(widget.Entry)
	
	// Create IconVariantEditor widget
	iconService := services.NewIconVariantService()
	itw["iconVariantEditor"] = custom_widgets.NewIconVariantEditor(
		"", // programName will be set when item is selected
		"", // itemName will be set when item is selected
		iconService,
		ui.Window,
		nil, // onVariantChange callback will be set in binders
	)
	
	itw[form] = widget.NewForm(
		widget.NewFormItem(name, itw[name]),
		widget.NewFormItem(cols, itw[cols]),
		widget.NewFormItem(rows, itw[rows]),
		widget.NewFormItem(tags, widget.NewEntry()),
		widget.NewFormItem("", itw[tags]),
		widget.NewFormItem(sm, itw[sm]),
		widget.NewFormItem("Icon Variants", itw["iconVariantEditor"]),
		// widget.NewFormItem(m, ui.EditorTabs.ItemsTab.Widgets[m]),
		// widget.NewFormItem("icons", container.NewGridWithRows(2, widget.NewIcon(theme.MediaFastForwardIcon()))),
	)
	itw[form].(*widget.Form).SubmitText = "Update"

	et.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(nil, nil, nil, nil, itw[acc]),
		container.NewBorder(nil, nil, nil, nil, itw[form]),
	)

	//===========================================================================================================POINTS
	ptw[acc] = widget.NewAccordion()
	ptw[name] = new(widget.Entry)
	ptw[x] = new(widget.Entry)
	ptw[y] = new(widget.Entry)
	ptw[form] = widget.NewForm(
		widget.NewFormItem(name, ptw[name]),
		widget.NewFormItem(x, ptw[x]),
		widget.NewFormItem(y, ptw[y]),
	)

	ptw[form].(*widget.Form).SubmitText = "Update"
	et.PointsTab.TabItem = NewEditorTab(
		"Points",
		container.NewBorder(nil, nil, nil, nil, ptw[acc]),
		container.NewBorder(nil, nil, nil, nil, ptw[form]),
	)

	//===========================================================================================================SEARCHAREAS
	satw[acc] = widget.NewAccordion()
	satw[name] = new(widget.Entry)
	satw[x1] = new(widget.Entry)
	satw[y1] = new(widget.Entry)
	satw[x2] = new(widget.Entry)
	satw[y2] = new(widget.Entry)
	satw[form] = widget.NewForm(
		widget.NewFormItem(name, satw[name]),
		widget.NewFormItem(x1, satw[x1]),
		widget.NewFormItem(y1, satw[y1]),
		widget.NewFormItem(x2, satw[x2]),
		widget.NewFormItem(y2, satw[y2]),
	)
	satw[form].(*widget.Form).SubmitText = "Update"
	et.SearchAreasTab.TabItem = NewEditorTab(
		"Search Areas",
		container.NewBorder(nil, nil, nil, nil, satw[acc]),
		container.NewBorder(nil, nil, nil, nil, satw[form]),
	)

	et.Append(et.ProgramsTab.TabItem)
	et.Append(et.ItemsTab.TabItem)
	et.Append(et.PointsTab.TabItem)
	et.Append(et.SearchAreasTab.TabItem)
}

func (u *Ui) constructNavButton() {
	u.EditorUi.NavButton.Text = "Back"
	u.EditorUi.NavButton.Icon = theme.NavigateBackIcon()
	u.EditorUi.NavButton.OnTapped = func() {
		u.Window.SetContent(fynetooltip.AddWindowToolTipLayer(u.MainUi.CanvasObject, u.Window.Canvas()))
	}
}

func (u *Ui) constructAddButton() {
	u.EditorUi.AddButton.Text = "New"
	u.EditorUi.AddButton.Icon = theme.ContentAddIcon()
	u.EditorUi.AddButton.Importance = widget.SuccessImportance

}

func (u *Ui) constructRemoveButton() {
	u.EditorUi.RemoveButton.Text = ""
	u.EditorUi.RemoveButton.Icon = theme.ContentRemoveIcon()
	u.EditorUi.RemoveButton.Importance = widget.DangerImportance
}
