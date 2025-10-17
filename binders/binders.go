package binders

import (
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
	"Squire/ui"
	"Squire/ui/custom_widgets"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
)

func UnbindAll() {
	ats := ui.GetUi().ActionTabs

	ats.BoundTimeEntry.Unbind()
	ats.BoundTimeSlider.Unbind()
}

func ResetBinds() {
	ats := ui.GetUi().ActionTabs

	ats.BoundWait = binding.BindStruct(actions.NewWait(int(ats.BoundTimeSlider.Value)))
	t, _ := ats.BoundWait.GetItem("Time")
	ats.BoundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))
	ats.BoundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))

	n, _ := ats.BoundPoint.GetValue("Name")
	x, _ := ats.BoundPoint.GetValue("X")
	y, _ := ats.BoundPoint.GetValue("Y")
	ats.BoundMove = binding.BindStruct(actions.NewMove(coordinates.Point{n.(string), x.(int), y.(int)}))

	ats.BoundKey = binding.BindStruct(actions.NewKey(ats.BoundKeySelect.Selected, actions.UpOrDown(ats.BoundStateToggle.Toggled)))
	k, _ := ats.BoundKey.GetItem("Key")
	s, _ := ats.BoundKey.GetItem("State")
	ats.BoundKeySelect.Bind(k.(binding.String))
	ats.BoundStateToggle.Bind(binding.StringToBoolWithFormat(s.(binding.String), "up|down"))

	ats.BoundClick = binding.BindStruct(actions.NewClick(actions.LeftOrRight(ats.BoundButtonToggle.Toggled)))
	b, _ := ats.BoundClick.GetItem("Button")
	ats.BoundButtonToggle.Bind(binding.StringToBoolWithFormat(b.(binding.String), "left|right"))

	ats.BoundLoop = binding.BindStruct(actions.NewLoop(int(ats.BoundCountSlider.Value), ats.BoundLoopNameEntry.Text, []actions.ActionInterface{}))
	c, _ := ats.BoundLoop.GetItem("Count")
	n, _ = ats.BoundLoop.GetItem("Name")
	ats.BoundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))
	// ats.BoundLoopNameEntry.Bind(n.(binding.String))

	//I could not figure out how to access the Name property of the nested AdvancedAction for ImageSearch
	ats.BoundImageSearch = binding.BindStruct(actions.NewImageSearch(ats.BoundImageSearchNameEntry.Text, []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95))
	ats.BoundImageSearchNameEntry.Unbind()

	// ats.BoundOcr = binding.BindStruct(actions.NewOcr(ats.BoundOcr.Text, []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95))
	// n, _ = ats.BoundImageSearch.GetItem("Name")
	// ats.BoundImageSearchNameEntry.Bind(n.(binding.String))

	// ui.GetUi().ActionTabs.BoundImageSearch = binding.BindStruct(actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95))

}

func SetActionTabBindings() {
	SetAccordionPointsLists(ui.GetUi().ActionTabs.PointsAccordion)
	SetAccordionSearchAreasLists(ui.GetUi().ActionTabs.SAAccordion)
	SetAccordionItemsLists(ui.GetUi().ActionTabs.ItemsAccordion)
	ui.GetUi().ActionTabs.BoundWait = binding.BindStruct(actions.NewWait(0))
	ui.GetUi().ActionTabs.BoundMove = binding.BindStruct(actions.NewMove(coordinates.Point{"blank", 0, 0}))
	ui.GetUi().ActionTabs.BoundKey = binding.BindStruct(actions.NewKey("ctrl", "down"))
	ui.GetUi().ActionTabs.BoundClick = binding.BindStruct(actions.NewClick("left"))
	ui.GetUi().ActionTabs.BoundLoop = binding.BindStruct(actions.NewLoop(1, "blank", []actions.ActionInterface{}))
	ui.GetUi().ActionTabs.BoundImageSearch = binding.BindStruct(actions.NewImageSearch("blank", []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95))
	ui.GetUi().ActionTabs.BoundSearchArea = binding.BindStruct(&coordinates.SearchArea{})
	ui.GetUi().ActionTabs.BoundPoint = binding.BindStruct(&coordinates.Point{Name: "template", X: 0, Y: 0})

	t, _ := ui.GetUi().ActionTabs.BoundWait.GetItem("Time")
	ui.GetUi().ActionTabs.BoundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
	ui.GetUi().ActionTabs.BoundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

}

func bindPointWidgets(di binding.Struct) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})

	ets := ui.GetUi().EditorTabs
	pt := ets.PointsTab.BindableWidgets

	name, _ := di.GetItem("Name")
	x, _ := di.GetItem("X")
	y, _ := di.GetItem("Y")

	pt["Name"].(*widget.Entry).Unbind()
	pt["X"].(*widget.Entry).Unbind()
	pt["Y"].(*widget.Entry).Unbind()
	x.RemoveListener(dl)
	y.RemoveListener(dl)

	pt["Name"].(*widget.Entry).Bind(name.(binding.String))
	pt["X"].(*widget.Entry).Bind(binding.IntToString(x.(binding.Int)))
	pt["Y"].(*widget.Entry).Bind(binding.IntToString(y.(binding.Int)))
	x.AddListener(dl)
	y.AddListener(dl)

}

func bindSearchAreaWidgets(di binding.Struct) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})

	ets := ui.GetUi().EditorTabs
	st := ets.SearchAreasTab.BindableWidgets

	name, _ := di.GetItem("Name")
	x1, _ := di.GetItem("LeftX")
	y1, _ := di.GetItem("TopY")
	x2, _ := di.GetItem("RightX")
	y2, _ := di.GetItem("BottomY")

	st["Name"].(*widget.Entry).Unbind()
	st["LeftX"].(*widget.Entry).Unbind()
	st["TopY"].(*widget.Entry).Unbind()
	st["RightX"].(*widget.Entry).Unbind()
	st["BottomY"].(*widget.Entry).Unbind()
	x1.RemoveListener(dl)
	y1.RemoveListener(dl)
	x2.RemoveListener(dl)
	y2.RemoveListener(dl)

	st["Name"].(*widget.Entry).Bind(name.(binding.String))
	st["LeftX"].(*widget.Entry).Bind(binding.IntToString(x1.(binding.Int)))
	st["TopY"].(*widget.Entry).Bind(binding.IntToString(y1.(binding.Int)))
	st["RightX"].(*widget.Entry).Bind(binding.IntToString(x2.(binding.Int)))
	st["BottomY"].(*widget.Entry).Bind(binding.IntToString(y2.(binding.Int)))
	x1.AddListener(dl)
	y1.AddListener(dl)
	x2.AddListener(dl)
	y2.AddListener(dl)

}

func (m *MacroBinding) bindAction(a actions.ActionInterface) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})
	ats := ui.GetUi().ActionTabs
	boundMacros[m.Name].BoundSelectedAction = binding.BindStruct(a)
	bsa := boundMacros[m.Name].BoundSelectedAction
	switch node := a.(type) {
	case *actions.Wait:
		ats.BoundWait = bsa
		// boundMacros[ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode] = binding.BindStruct(node)
		t, _ := ats.BoundWait.GetItem("Time")

		ats.BoundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
		ats.BoundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

		t.AddListener(dl)
	case *actions.Move:
		ats.BoundMove = bsa
	case *actions.Click:
		ats.BoundClick = bsa
		b, _ := ats.BoundClick.GetItem("Button")
		ats.BoundButtonToggle.Bind(custom_widgets.CustomStringToBool(b.(binding.String), "click", dl))
	case *actions.Key:
		ats.BoundKey = bsa
		k, _ := ats.BoundKey.GetItem("Key")
		s, _ := ats.BoundKey.GetItem("State")

		ats.BoundKeySelect.Bind(k.(binding.String))
		ats.BoundStateToggle.Bind(custom_widgets.CustomStringToBool(s.(binding.String), "key", dl))

		k.AddListener(dl)
		s.AddListener(dl)

	case *actions.Loop:
		ats.BoundLoop = bsa
		ats.BoundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		c, _ := ats.BoundLoop.GetItem("Count")
		n, _ := ats.BoundAdvancedAction.GetItem("Name")

		ats.BoundLoopNameEntry.Bind(n.(binding.String))
		ats.BoundCountLabel.Bind(binding.IntToString(c.(binding.Int)))
		ats.BoundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))

		c.AddListener(dl)
		n.AddListener(dl)
	case *actions.ImageSearch:
		ats.BoundImageSearch = bsa
		ats.BoundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		ats.BoundSearchArea = binding.BindStruct(&node.SearchArea)

		n, _ := ats.BoundAdvancedAction.GetItem("Name")
		sa, _ := ats.BoundSearchArea.GetItem("Name")
		ts, _ := ats.BoundImageSearch.GetItem("Targets")

		ats.BoundImageSearchNameEntry.Bind(n.(binding.String))

		ats.BoundImageSearch.SetValue("Targets", slices.Clone(node.Targets))
		for _, ai := range ats.ItemsAccordion.Items {
			ai.Detail.Refresh()
		}

		ts.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	case *actions.Ocr:
		ats.BoundOcr = bsa
		ats.BoundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		ats.BoundSearchArea = binding.BindStruct(&node.SearchArea)

		t, _ := ats.BoundOcr.GetItem("Target")
		n, _ := ats.BoundAdvancedAction.GetItem("Name")
		sa, _ := ats.BoundSearchArea.GetItem("Name")

		t.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	}
}
