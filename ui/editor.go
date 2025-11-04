package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

type EditorUi struct {
	fyne.CanvasObject
	NavButton       *widget.Button
	AddButton       *widget.Button
	RemoveButton    *widget.Button
	ProgramSelector *widget.SelectEntry
	EditorTabs      struct {
		*container.AppTabs
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
		name   = "Name"
		x      = "X"
		y      = "Y"
		x1     = "RightX"
		y1     = "TopY"
		x2     = "LeftX"
		y2     = "BottomY"
		gsx    = "GridSizeX"
		gsy    = "GridSizeY"
		tags   = "Tags"
		sm     = "StackMax"
		m      = "Merchant"
		form   = "Form"
		points = "Points"
		// i    = "Icons"
	)
	ui.EditorTabs.ItemsTab.Widgets[name] = new(widget.Entry)
	ui.EditorTabs.ItemsTab.Widgets[gsx] = new(widget.Entry)
	ui.EditorTabs.ItemsTab.Widgets[gsy] = new(widget.Entry)
	ui.EditorTabs.ItemsTab.Widgets[tags] = widget.NewCard("test", "", nil)
	ui.EditorTabs.ItemsTab.Widgets[sm] = new(widget.Entry)
	ui.EditorTabs.ItemsTab.Widgets[m] = widget.NewEntryWithData(binding.NewString())
	// ui.EditorTabs.ItemsTab.BindableWidgets[i] = container.NewGridWithRows(2, )
	ui.EditorTabs.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(
			nil, nil, nil, nil, widget.NewAccordion(),
		),
		container.NewBorder(
			nil, nil,
			nil, nil,
			widget.NewForm(
				widget.NewFormItem(name, ui.EditorTabs.ItemsTab.Widgets[name]),
				widget.NewFormItem(gsx, ui.EditorTabs.ItemsTab.Widgets[gsx]),
				widget.NewFormItem(gsy, ui.EditorTabs.ItemsTab.Widgets[gsy]),
				widget.NewFormItem(tags, widget.NewEntry()),
				widget.NewFormItem("", ui.EditorTabs.ItemsTab.Widgets[tags]),
				widget.NewFormItem(sm, ui.EditorTabs.ItemsTab.Widgets[sm]),
				widget.NewFormItem(m, ui.EditorTabs.ItemsTab.Widgets[m]),
				// widget.NewFormItem("icons", container.NewGridWithRows(2, widget.NewIcon(theme.MediaFastForwardIcon()))),
			)),
	)

	ui.EditorTabs.PointsTab.Widgets[points] = widget.NewAccordion()
	ui.EditorTabs.PointsTab.Widgets[name] = new(widget.Entry)
	ui.EditorTabs.PointsTab.Widgets[x] = new(widget.Entry)
	ui.EditorTabs.PointsTab.Widgets[y] = new(widget.Entry)
	ui.EditorTabs.PointsTab.Widgets[form] = widget.NewForm(
		widget.NewFormItem(name, ui.EditorTabs.PointsTab.Widgets[name]),
		widget.NewFormItem(x, ui.EditorTabs.PointsTab.Widgets[x]),
		widget.NewFormItem(y, ui.EditorTabs.PointsTab.Widgets[y]),
	)
	// pointsForm.OnSubmit = func() {

	// }
	ui.EditorTabs.PointsTab.Widgets[form].(*widget.Form).SubmitText = "Update"
	ui.EditorUi.EditorTabs.PointsTab.TabItem = NewEditorTab(
		"Points",
		container.NewBorder(nil, nil, nil, nil, ui.EditorTabs.PointsTab.Widgets[points]),
		container.NewBorder(nil, nil, nil, nil, ui.EditorTabs.PointsTab.Widgets[form]),
	)

	ui.EditorTabs.SearchAreasTab.Widgets[name] = new(widget.Entry)
	ui.EditorTabs.SearchAreasTab.Widgets[x1] = new(widget.Entry)
	ui.EditorTabs.SearchAreasTab.Widgets[y1] = new(widget.Entry)
	ui.EditorTabs.SearchAreasTab.Widgets[x2] = new(widget.Entry)
	ui.EditorTabs.SearchAreasTab.Widgets[y2] = new(widget.Entry)
	ui.EditorUi.EditorTabs.SearchAreasTab.TabItem = NewEditorTab(
		"Search Areas",
		container.NewBorder(nil, nil, nil, nil, widget.NewAccordion()),
		container.NewBorder(nil, nil, nil, nil, widget.NewForm(
			widget.NewFormItem(name, ui.EditorTabs.SearchAreasTab.Widgets[name]),
			widget.NewFormItem(x1, container.NewGridWithColumns(2,
				ui.EditorTabs.SearchAreasTab.Widgets[x1])),
			widget.NewFormItem(y1, ui.EditorTabs.SearchAreasTab.Widgets[y1]),
			widget.NewFormItem(x2, ui.EditorTabs.SearchAreasTab.Widgets[x2]),
			widget.NewFormItem(y2, ui.EditorTabs.SearchAreasTab.Widgets[y2]),
		)),
	)

	ui.EditorUi.EditorTabs.Append(ui.EditorUi.EditorTabs.ItemsTab.TabItem)
	ui.EditorUi.EditorTabs.Append(ui.EditorUi.EditorTabs.PointsTab.TabItem)
	ui.EditorUi.EditorTabs.Append(ui.EditorUi.EditorTabs.SearchAreasTab.TabItem)
}

func (u *Ui) constructNavButton() {
	u.EditorUi.NavButton.Text = "Back"
	u.EditorUi.NavButton.Icon = theme.NavigateBackIcon()
	u.EditorUi.NavButton.OnTapped = func() {
		u.Window.SetContent(u.MainUi.CanvasObject)
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
