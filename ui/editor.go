package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
)

type EditorUi struct {
	Window fyne.Window

	EditorTabs struct {
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

func constructEditorWindow() {
	ConstructEditorTabs()
	ui.EditorUi.Window.SetContent(ui.EditorUi.EditorTabs)
}

// I need to complete the structure of the editor window here and then complete the bindings in binders. add as many properties to EditorUi as u need bro
func ConstructEditorTabs() {
	var (
		name = "Name"
		x    = "X"
		y    = "Y"
		x1   = "RightX"
		y1   = "TopY"
		x2   = "LeftX"
		y2   = "BottomY"
		gs   = "GridSize"
		tags = "Tags"
		sm   = "StackMax"
		m    = "Merchant"
	)

	ui.EditorTabs.ItemsTab.BindableWidgets[name] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[gs] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[tags] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[sm] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.BindableWidgets[m] = widget.NewEntryWithData(binding.NewString())
	ui.EditorTabs.ItemsTab.TabItem = NewEditorTab(
		"Items",
		container.NewBorder(nil, nil, nil, nil, widget.NewAccordion()),
		container.NewBorder(nil, nil, nil, nil, widget.NewForm(
			widget.NewFormItem(name, ui.EditorTabs.ItemsTab.BindableWidgets[name]),
			widget.NewFormItem(gs, ui.EditorTabs.ItemsTab.BindableWidgets[gs]),
			widget.NewFormItem(tags, ui.EditorTabs.ItemsTab.BindableWidgets[tags]),
			widget.NewFormItem(sm, ui.EditorTabs.ItemsTab.BindableWidgets[sm]),
			widget.NewFormItem(m, ui.EditorTabs.ItemsTab.BindableWidgets[m]),
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
			widget.NewFormItem(x1, ui.EditorTabs.SearchAreasTab.BindableWidgets[x1]),
			widget.NewFormItem(y1, ui.EditorTabs.SearchAreasTab.BindableWidgets[y1]),
			widget.NewFormItem(x2, ui.EditorTabs.SearchAreasTab.BindableWidgets[x2]),
			widget.NewFormItem(y2, ui.EditorTabs.SearchAreasTab.BindableWidgets[y2]),
		)),
	)

	ui.EditorUi.EditorTabs.Append(ui.EditorUi.EditorTabs.ItemsTab.TabItem)
	ui.EditorUi.EditorTabs.Append(ui.EditorUi.EditorTabs.PointsTab.TabItem)
	ui.EditorUi.EditorTabs.Append(ui.EditorUi.EditorTabs.SearchAreasTab.TabItem)
}

func launchEditorWindow() {

	// 	i := &items.Item{}
	// 	bi := binding.BindStruct(i)
	// 	n, _ := bi.GetItem("Name")
	// 	gs, _ := bi.GetItem("GridSize")
	// 	// sm, _ := bi.GetItem("StackMax")
	// 	m, _ := bi.GetItem("Merchant")
	// form := widget.NewForm(
	// 	widget.NewFormItem("Name:", widget.NewEntryWithData(n.(binding.String))),
	// 	widget.NewFormItem("Max Stacksize:", widget.NewEntryWithData(binding.IntToString(gs.(binding.Int)))),
	// 	widget.NewFormItem("Grid Size:", container.NewHBox(widget.NewLabel("Width: "), widget.NewEntry(), widget.NewLabel("Height: "), widget.NewEntry())),
	// 	widget.NewFormItem("Merchant:", widget.NewEntryWithData(m.(binding.String))),
	// )
	// 	// nv, _ := bi.GetValue("Name")

	// 	form.OnSubmit = func() {
	// 		// GetUi().p.Items[nv.(string)] = *i
	// 	}
	ui.EditorUi.Window.Show()
}
