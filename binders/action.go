package binders

// import (
// 	"Squire/internal/models/actions"
// 	"Squire/ui"
// 	"slices"
// 	"fyne.io/fyne/v2"
// 	"fyne.io/fyne/v2/data/binding"
// )

// func InitBinds() {
// 	ats := ui.GetUi().ActionTabs

// 	ats.BoundWait = binding.BindStruct(actions.NewWait(0))
// 	m := actions.NewMove(actions.Point{Name: "blank", X: 0, Y: 0})
// 	ats.BoundMove = binding.BindStruct(m)
// 	ats.BoundPoint = binding.BindStruct(&m.Point)
// 	ats.BoundKey = binding.BindStruct(actions.NewKey("ctrl", true))
// 	ats.BoundClick = binding.BindStruct(actions.NewClick(false))

// 	l := actions.NewLoop(1, "blank", []actions.ActionInterface{})
// 	ats.BoundLoop = binding.BindStruct(l)
// 	ats.BoundLoopAA = binding.BindStruct(l.AdvancedAction)
// 	is := actions.NewImageSearch("blank", []actions.ActionInterface{}, []string{}, actions.SearchArea{}, 1, 1, 0.95)
// 	ats.BoundImageSearch = binding.BindStruct(is)
// 	ats.BoundImageSearchAA = binding.BindStruct(is.AdvancedAction)
// 	ats.BoundImageSearchSA = binding.BindStruct(&is.SearchArea)
// 	ocr := actions.NewOcr("blank", []actions.ActionInterface{}, "blank", actions.SearchArea{})
// 	ats.BoundOcr = binding.BindStruct(ocr)
// 	ats.BoundOcrAA = binding.BindStruct(ocr.AdvancedAction)
// 	ats.BoundOcrSA = binding.BindStruct(&ocr.SearchArea)

// }

// func ResetBinds() {
// 	ats := ui.GetUi().ActionTabs
// 	bindAction(actions.NewWait(int(ats.BoundTimeSlider.Value)))
// 	n, _ := ats.BoundPoint.GetValue("Name")
// 	x, _ := ats.BoundPoint.GetValue("X")
// 	y, _ := ats.BoundPoint.GetValue("Y")
// 	bindAction(actions.NewMove(actions.Point{n.(string), x.(int), y.(int)})) //n.(string), x.(int), y.(int)}))
// 	bindAction(actions.NewKey(ats.BoundKeySelect.Selected, ats.BoundStateToggle.Toggled))
// 	bindAction(actions.NewClick(ats.BoundButtonToggle.Toggled))
// 	bindAction(actions.NewLoop(int(ats.BoundCountSlider.Value), ats.BoundLoopNameEntry.Text, []actions.ActionInterface{}))
// 	bindAction(actions.NewImageSearch(ats.BoundImageSearchNameEntry.Text, []actions.ActionInterface{}, []string{}, actions.SearchArea{}, int(ats.BoundImageSearchRowSplitSlider.Value), int(ats.BoundImageSearchColSplitSlider.Value), 0.95))
// 	bindAction(actions.NewOcr(ats.BoundOcrNameEntry.Text, []actions.ActionInterface{}, ats.BoundOcrTargetEntry.Text, actions.SearchArea{}))
// }

// func SetActionTabBindings() {
// 	ats := ui.GetUi().ActionTabs
// 	ResetBinds()
// 	setAccordionPointsLists(ats.PointsAccordion)
// 	setAccordionSearchAreasLists(ats.ImageSearchSAAccordion)
// 	setAccordionItemsLists(ats.ImageSearchItemsAccordion)
// 	setAccordionSearchAreasLists(ats.OcrSAAccordion)
// }

// func bindAction(a actions.ActionInterface) {
// 	dl := binding.NewDataListener(func() {
// 		mt := ui.GetUi().Mui.MTabs.SelectedTab()
// 		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
// 	})
// 	ats := ui.GetUi().ActionTabs
// 	bsa := binding.BindStruct(a)
// 	switch node := a.(type) {
// 	case *actions.Wait:
// 		ats.BoundWait = bsa
// 		t, _ := ats.BoundWait.GetItem("Time")

// 		ats.BoundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
// 		ats.BoundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

// 		t.AddListener(dl)
// 	case *actions.Move:
// 		ats.BoundMove = bsa
// 		ats.BoundPoint = binding.BindStruct(&node.Point)
// 	case *actions.Click:
// 		ats.BoundClick = bsa
// 		b, _ := ats.BoundClick.GetItem("Button")
// 		ats.BoundButtonToggle.Bind(b.(binding.Bool))

// 		b.AddListener(dl)
// 	case *actions.Key:
// 		ats.BoundKey = bsa
// 		k, _ := ats.BoundKey.GetItem("Key")
// 		s, _ := ats.BoundKey.GetItem("State")

// 		ats.BoundKeySelect.Bind(k.(binding.String))
// 		ats.BoundStateToggle.Bind(s.(binding.Bool))

// 		k.AddListener(dl)
// 		s.AddListener(dl)

// 	case *actions.Loop:
// 		ats.BoundLoop = bsa
// 		ats.BoundLoopAA = binding.BindStruct(node.AdvancedAction)
// 		c, _ := ats.BoundLoop.GetItem("Count")
// 		n, _ := ats.BoundLoopAA.GetItem("Name")

// 		ats.BoundLoopNameEntry.Bind(n.(binding.String))
// 		ats.BoundCountLabel.Bind(binding.IntToString(c.(binding.Int)))
// 		ats.BoundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))

// 		c.AddListener(dl)
// 		n.AddListener(dl)
// 	case *actions.ImageSearch:
// 		ats.BoundImageSearch = bsa
// 		ats.BoundImageSearchAA = binding.BindStruct(node.AdvancedAction)
// 		ats.BoundImageSearchSA = binding.BindStruct(&node.SearchArea)
// 		n, _ := ats.BoundImageSearchAA.GetItem("Name")
// 		sa, _ := ats.BoundImageSearchSA.GetItem("Name")
// 		ts, _ := ats.BoundImageSearch.GetItem("Targets")
// 		rs, _ := ats.BoundImageSearch.GetItem("RowSplit")
// 		cs, _ := ats.BoundImageSearch.GetItem("ColSplit")

// 		ats.BoundImageSearchNameEntry.Bind(n.(binding.String))
// 		ats.BoundImageSearchColSplitSlider.Bind(binding.IntToFloat(rs.(binding.Int)))
// 		ats.BoundImageSearchRowSplitSlider.Bind(binding.IntToFloat(cs.(binding.Int)))
// 		ats.BoundImageSearchColSplitLabel.Bind(binding.IntToString(rs.(binding.Int)))
// 		ats.BoundImageSearchRowSplitLabel.Bind(binding.IntToString(cs.(binding.Int)))
// 		ats.BoundImageSearch.SetValue("Targets", slices.Clone(node.Targets))
// 		RefreshItemsAccordionItems()

// 		ts.AddListener(dl)
// 		n.AddListener(dl)
// 		sa.AddListener(dl)
// 		rs.AddListener(dl)
// 		cs.AddListener(dl)
// 	case *actions.Ocr:
// 		ats.BoundOcr = bsa
// 		ats.BoundOcrAA = binding.BindStruct(node.AdvancedAction)
// 		ats.BoundOcrSA = binding.BindStruct(&node.SearchArea)

// 		n, _ := ats.BoundOcrAA.GetItem("Name")
// 		t, _ := ats.BoundOcr.GetItem("Target")
// 		sa, _ := ats.BoundOcrSA.GetItem("Name")
// 		ats.BoundOcrNameEntry.Bind(n.(binding.String))
// 		ats.BoundOcrTargetEntry.Bind(t.(binding.String))

// 		t.AddListener(dl)
// 		n.AddListener(dl)
// 		sa.AddListener(dl)
// 	}
// 	ui.GetUi().Mui.MTabs.SelectedTab().Refresh() //could probably just refresh the selected tree item here
// }
