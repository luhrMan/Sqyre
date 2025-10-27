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

	BindableWidgets map[string]fyne.Widget
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
		gsx  = "GridSizeX"
		gsy  = "GridSizeY"
		tags = "Tags"
		sm   = "StackMax"
		m    = "Merchant"
		// i    = "Icons"
	)
	ui.EditorTabs.ItemsTab.BindableWidgets[name] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[gsx] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[gsy] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[tags] = widget.NewCard("test", "", nil)
	ui.EditorTabs.ItemsTab.BindableWidgets[sm] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[m] = widget.NewEntryWithData(binding.NewString())
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
				widget.NewFormItem(name, ui.EditorTabs.ItemsTab.BindableWidgets[name]),
				widget.NewFormItem(gsx, ui.EditorTabs.ItemsTab.BindableWidgets[gsx]),
				widget.NewFormItem(gsy, ui.EditorTabs.ItemsTab.BindableWidgets[gsy]),
				widget.NewFormItem(tags, widget.NewEntry()),
				widget.NewFormItem("", ui.EditorTabs.ItemsTab.BindableWidgets[tags]),
				widget.NewFormItem(sm, ui.EditorTabs.ItemsTab.BindableWidgets[sm]),
				widget.NewFormItem(m, ui.EditorTabs.ItemsTab.BindableWidgets[m]),
				// widget.NewFormItem("icons", container.NewGridWithRows(2, widget.NewIcon(theme.MediaFastForwardIcon()))),
			)),
	)

	ui.EditorTabs.PointsTab.BindableWidgets[name] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.PointsTab.BindableWidgets[x] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.PointsTab.BindableWidgets[y] = widget.NewEntryWithData(binding.NewString())
	ui.EditorUi.EditorTabs.PointsTab.TabItem = NewEditorTab(
		"Points",
		container.NewBorder(nil, nil, nil, nil, widget.NewAccordion()),
		container.NewBorder(nil, nil, nil, nil, widget.NewForm(
			widget.NewFormItem(name, ui.EditorTabs.PointsTab.BindableWidgets[name]),
			widget.NewFormItem(x, ui.EditorTabs.PointsTab.BindableWidgets[x]),
			widget.NewFormItem(y, ui.EditorTabs.PointsTab.BindableWidgets[y]),
		)),
	)

	ui.EditorTabs.SearchAreasTab.BindableWidgets[name] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.SearchAreasTab.BindableWidgets[x1] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.SearchAreasTab.BindableWidgets[y1] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.SearchAreasTab.BindableWidgets[x2] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.SearchAreasTab.BindableWidgets[y2] = widget.NewEntryWithData(binding.NewString())
	ui.EditorUi.EditorTabs.SearchAreasTab.TabItem = NewEditorTab(
		"Search Areas",
		container.NewBorder(nil, nil, nil, nil, widget.NewAccordion()),
		container.NewBorder(nil, nil, nil, nil, widget.NewForm(
			widget.NewFormItem(name, ui.EditorTabs.SearchAreasTab.BindableWidgets[name]),

			widget.NewFormItem(x1, container.NewGridWithColumns(2,
				ui.EditorTabs.SearchAreasTab.BindableWidgets[x1])),
			widget.NewFormItem(y1, ui.EditorTabs.SearchAreasTab.BindableWidgets[y1]),
			widget.NewFormItem(x2, ui.EditorTabs.SearchAreasTab.BindableWidgets[x2]),
			widget.NewFormItem(y2, ui.EditorTabs.SearchAreasTab.BindableWidgets[y2]),
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
