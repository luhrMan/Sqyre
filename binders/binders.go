package binders

import (
	"Squire/internal/models/actions"
	"Squire/internal/models/coordinates"
	"Squire/ui"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/data/binding"
)

func initBinds() {
	ui.GetUi().ActionTabs.BoundWait = binding.BindStruct(actions.NewWait(0))
	ui.GetUi().ActionTabs.BoundMove = binding.BindStruct(actions.NewMove(coordinates.Point{"blank", 0, 0}))
	ui.GetUi().ActionTabs.BoundKey = binding.BindStruct(actions.NewKey("ctrl", true))
	ui.GetUi().ActionTabs.BoundClick = binding.BindStruct(actions.NewClick(false))

	l := actions.NewLoop(1, "blank", []actions.ActionInterface{})
	ui.GetUi().ActionTabs.BoundLoop = binding.BindStruct(l)
	ui.GetUi().ActionTabs.BoundLoopAA = binding.BindStruct(l.AdvancedAction)
	is := actions.NewImageSearch("blank", []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95)
	ui.GetUi().ActionTabs.BoundImageSearch = binding.BindStruct(is)
	ui.GetUi().ActionTabs.BoundImageSearchAA = binding.BindStruct(is.AdvancedAction)
	ui.GetUi().ActionTabs.BoundImageSearchSA = binding.BindStruct(is.SearchArea)
	ocr := actions.NewOcr("blank", []actions.ActionInterface{}, "blank", coordinates.SearchArea{})
	ui.GetUi().ActionTabs.BoundOcr = binding.BindStruct(ocr)
	ui.GetUi().ActionTabs.BoundOcrAA = binding.BindStruct(ocr.AdvancedAction)
	ui.GetUi().ActionTabs.BoundOcrSA = binding.BindStruct(ocr.SearchArea)

	ui.GetUi().ActionTabs.BoundPoint = binding.BindStruct(&coordinates.Point{Name: "template", X: 0, Y: 0})

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

	ats.BoundKey = binding.BindStruct(actions.NewKey(ats.BoundKeySelect.Selected, ats.BoundStateToggle.Toggled))
	k, _ := ats.BoundKey.GetItem("Key")
	s, _ := ats.BoundKey.GetItem("State")
	ats.BoundKeySelect.Bind(k.(binding.String))
	ats.BoundStateToggle.Bind(s.(binding.Bool))

	ats.BoundClick = binding.BindStruct(actions.NewClick(ats.BoundButtonToggle.Toggled))
	b, _ := ats.BoundClick.GetItem("Button")
	ats.BoundButtonToggle.Bind(b.(binding.Bool))

	l := actions.NewLoop(int(ats.BoundCountSlider.Value), ats.BoundLoopNameEntry.Text, []actions.ActionInterface{})
	ats.BoundLoop = binding.BindStruct(l)
	ats.BoundLoopAA = binding.BindStruct(l.AdvancedAction)
	c, _ := ats.BoundLoop.GetItem("Count")
	n, _ = ats.BoundLoopAA.GetItem("Name")
	ats.BoundLoopNameEntry.Bind(n.(binding.String))
	ats.BoundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))
	ats.BoundCountLabel.Bind(binding.IntToString(c.(binding.Int)))

	is := actions.NewImageSearch(ats.BoundImageSearchNameEntry.Text, []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95)
	ats.BoundImageSearch = binding.BindStruct(is)
	ats.BoundImageSearchAA = binding.BindStruct(is.AdvancedAction)
	n, _ = ats.BoundImageSearchAA.GetItem("Name")
	t, _ = ats.BoundImageSearch.GetItem("Targets")
	t.AddListener(binding.NewDataListener(func() {

	}))

	ats.BoundImageSearchNameEntry.Bind(n.(binding.String))

	// ats.BoundOcr = binding.BindStruct(actions.NewOcr(ats.BoundOcr.Text, []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95))
	// n, _ = ats.BoundImageSearch.GetItem("Name")
	// ats.BoundImageSearchNameEntry.Bind(n.(binding.String))

	// ui.GetUi().ActionTabs.BoundImageSearch = binding.BindStruct(actions.NewImageSearch("", []actions.ActionInterface{}, []string{}, coordinates.SearchArea{}, 1, 1, 0.95))

}

func SetActionTabBindings() {
	initBinds()
	ResetBinds()
	setAccordionPointsLists(ui.GetUi().ActionTabs.PointsAccordion)
	setAccordionSearchAreasLists(ui.GetUi().ActionTabs.ImageSearchSAAccordion)
	setAccordionItemsLists(ui.GetUi().ActionTabs.ImageSearchItemsAccordion)
}

func bindAction(a actions.ActionInterface) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})
	ats := ui.GetUi().ActionTabs
	bsa := binding.BindStruct(a)
	switch node := a.(type) {
	case *actions.Wait:
		ats.BoundWait = bsa
		t, _ := ats.BoundWait.GetItem("Time")

		ats.BoundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
		ats.BoundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

		t.AddListener(dl)
	case *actions.Move:
		ats.BoundMove = bsa
	case *actions.Click:
		ats.BoundClick = bsa
		b, _ := ats.BoundClick.GetItem("Button")
		ats.BoundButtonToggle.Bind(b.(binding.Bool))

		b.RemoveListener(dl)
		b.AddListener(dl)
	case *actions.Key:
		ats.BoundKey = bsa
		k, _ := ats.BoundKey.GetItem("Key")
		s, _ := ats.BoundKey.GetItem("State")

		ats.BoundKeySelect.Bind(k.(binding.String))
		ats.BoundStateToggle.Bind(s.(binding.Bool))

		k.AddListener(dl)
		s.AddListener(dl)

	case *actions.Loop:
		ats.BoundLoop = bsa
		ats.BoundLoopAA = binding.BindStruct(node.AdvancedAction)
		c, _ := ats.BoundLoop.GetItem("Count")
		n, _ := ats.BoundLoopAA.GetItem("Name")

		ats.BoundLoopNameEntry.Bind(n.(binding.String))
		ats.BoundCountLabel.Bind(binding.IntToString(c.(binding.Int)))
		ats.BoundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))

		c.AddListener(dl)
		n.AddListener(dl)
	case *actions.ImageSearch:
		ats.BoundImageSearch = bsa
		ats.BoundImageSearchAA = binding.BindStruct(node.AdvancedAction)
		ats.BoundImageSearchSA = binding.BindStruct(&node.SearchArea)

		n, _ := ats.BoundImageSearchAA.GetItem("Name")
		sa, _ := ats.BoundImageSearchSA.GetItem("Name")
		ts, _ := ats.BoundImageSearch.GetItem("Targets")

		ats.BoundImageSearchNameEntry.Bind(n.(binding.String))

		ats.BoundImageSearch.SetValue("Targets", slices.Clone(node.Targets))
		RefreshItemsAccordionItems()

		ts.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	case *actions.Ocr:
		ats.BoundOcr = bsa
		ats.BoundOcrAA = binding.BindStruct(node.AdvancedAction)
		ats.BoundOcrSA = binding.BindStruct(&node.SearchArea)

		t, _ := ats.BoundOcr.GetItem("Target")
		n, _ := ats.BoundOcrAA.GetItem("Name")
		sa, _ := ats.BoundOcrSA.GetItem("Name")

		t.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	}
}
