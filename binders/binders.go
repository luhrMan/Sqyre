package binders

import (
	"Squire/internal/config"
	model "Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/macro"
	"Squire/ui"
	"Squire/ui/custom_widgets"
	"log"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func InitPrograms() {
	once.Do(func() {
		programs = model.GetPrograms()
	})
}

func GetProgram(s string) *model.Program {
	if p, ok := GetPrograms()[s]; ok {
		return p
	}
	return nil
}

func GetPrograms() map[string]*model.Program {
	return programs
}

func GetItemsAsTree() map[string][]string {
	newMap := map[string][]string{
		"": {config.DarkAndDarker},
	}

	// for key, program := range GetPrograms() {
	p := GetProgram(config.DarkAndDarker)
	strSlice := []string{}
	// strSlice = append(strSlice, p.Name)
	// for _, macro := range program.Macros {
	// 	strSlice = append(strSlice, macro.Name)
	// }

	// for _, item := range p.Items {
	// 	strSlice = append(strSlice, item.Name)
	// }
	keystr := config.DarkAndDarker
	newMap[keystr] = append(newMap[keystr], "coordinates")
	for ss, coordinates := range p.Coordinates {
		newMap["coordinates"] = append(newMap["coordinates"], ss)
		newMap[ss] = append(newMap[ss], "points")
		strSlice = []string{}
		for _, point := range coordinates.Points {
			strSlice = append(strSlice, point.Name)
		}
		newMap["points"] = strSlice
		strSlice = []string{}
		newMap[ss] = append(newMap[ss], "search areas")
		for _, searchArea := range coordinates.SearchAreas {
			strSlice = append(strSlice, searchArea.Name)
		}
	}
	return newMap
}

func GetMacros() []*macro.Macro {
	return macros
}

func GetMacro(s string) *macro.Macro {
	for _, m := range GetMacros() {
		if m.Name == s {
			return m
		}
	}
	return nil
}

func AddMacro(s string, d int) {
	if s == "" {
		return
	}
	macros = append(macros, macro.NewMacro(s, d, []string{}))
}

func GetPointsAsStringSlice(program string, ss string) []string {
	p := GetProgram(program)
	keys := make([]string, len(p.Coordinates[ss].Points))
	i := 0
	for k := range p.Coordinates[ss].Points {
		keys[i] = k
		i++
	}

	return keys
}

func SetPointsLists(acc *widget.Accordion) {
	// for i, t := range at.AppTabs.Items {
	// 	if t.Text == "Move" {
	// 		t.Content.(*widget.Accordion).Items[i].Detail.(*fyne.Container).Objects[1].(*widget.List).OnSelected = func(lii widget.ListItemID) {
	// 			v, _ := boundMovePointStringList.GetValue(lii)
	// 			point := binders.GetProgram(key).Coordinates[config.MainMonitorSizeString].Points[v]
	// 			binders.BindActionMove(point)
	// 			// at.BoundMove.SetValue("Point", p)
	// 			// at.BoundPoint.SetValue("X", p.X)
	// 			// at.BoundPoint.SetValue("Y", p.Y)
	// 			at.BoundMove.Reload()
	// 			GetUi().mui.mtabs.selectedTab().Refresh()
	// 		}
	// 	}
	// }
	for key, pro := range GetPrograms() {
		lists := struct {
			boundMovePointStringList binding.ExternalStringList
			boundMovePointSearchBar  *widget.Entry
			boundPointList           *widget.List
			pointSearchList          []string
		}{
			boundMovePointStringList: binding.BindStringList(&[]string{}),
			boundMovePointSearchBar:  &widget.Entry{},
			boundPointList:           &widget.List{},
			pointSearchList:          slices.Clone(GetPointsAsStringSlice(key, config.MainMonitorSizeString)),
		}
		lists.boundMovePointStringList = binding.BindStringList(&lists.pointSearchList)
		lists.boundPointList = widget.NewListWithData(
			lists.boundMovePointStringList,
			func() fyne.CanvasObject {
				return widget.NewLabel("template") //container.NewBorder(nil, nil, nil, nil, widget.NewLabel("template"), widget.NewLabel("template"))
			},
			func(di binding.DataItem, co fyne.CanvasObject) {
				bsa := di.(binding.String)
				label := co.(*widget.Label)
				// nameLabel := c.Objects[0].(*widget.Label)
				// coordsLabel := c.Objects[1].(*widget.Label)
				v, _ := bsa.Get()
				poi := pro.Coordinates[config.MainMonitorSizeString].GetPoint(v)
				// t := fmt.Sprintf("%v| %d, %d", poi.Name, poi.X, poi.Y)
				label.Bind(binding.BindString(&poi.Name))
				// label.SetText((fmt.Sprintf("%v: %d, %d", poi.Name, poi.X, poi.Y)))
			},
		)

		lists.boundPointList.OnSelected = func(lii widget.ListItemID) {
			v, e := lists.boundMovePointStringList.GetValue(lii)
			if e != nil {
				log.Println(e)
			}
			//strings.TrimSuffix("|", v)

			p := pro.Coordinates[config.MainMonitorSizeString].GetPoint(v)
			log.Println(p)
			// bindPointToMoveWidgets(&p)
			// at.boundImageSearch.SetValue("SearchArea", programs.CurrentProgramAndScreenSizeCoordinates().SearchAreas[v])
			// at.boundSearchArea.SetValue("Name", v)
			// at.boundImageSearch.Reload()
			// GetUi().mui.mtabs.selectedTab().Refresh()
		}
		lists.boundMovePointSearchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := GetPointsAsStringSlice(key, config.MainMonitorSizeString)
				defer lists.boundMovePointStringList.Reload()
				defer lists.boundPointList.ScrollToTop()
				defer lists.boundPointList.Refresh()

				if s == "" {
					lists.pointSearchList = defaultList
					return
				}
				lists.pointSearchList = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.pointSearchList = append(lists.pointSearchList, i)
					}
				}
			},
		}
		programPointListWidget := *widget.NewAccordionItem(
			pro.Name,
			container.NewBorder(
				lists.boundMovePointSearchBar,
				nil, nil, nil,
				lists.boundPointList,
			),
		)
		acc.Append(&programPointListWidget)
	}
}
func bindPointAndNode(p *coordinates.Point) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})
	ats := ui.GetUi().ActionTabs

	m := actions.NewMove(*p)

	// ats.BoundMove = nil
	// ats.BoundPoint = nil

	ats.BoundMove = binding.BindStruct(m)
	ats.BoundPoint = binding.BindStruct(p)
	x, _ := ats.BoundPoint.GetItem("X")
	y, _ := ats.BoundPoint.GetItem("Y")

	ats.BoundMoveXSlider.Unbind()
	ats.BoundMoveYSlider.Unbind()
	ats.BoundMoveXEntry.Unbind()
	ats.BoundMoveYEntry.Unbind()
	ats.BoundMoveXSlider.Bind(binding.IntToFloat(x.(binding.Int)))
	ats.BoundMoveYSlider.Bind(binding.IntToFloat(y.(binding.Int)))
	ats.BoundMoveXEntry.Bind(binding.IntToString(x.(binding.Int)))
	ats.BoundMoveYEntry.Bind(binding.IntToString(y.(binding.Int)))
	// ats.boundSpotSelect.Bind())

	x.AddListener(dl)
	y.AddListener(dl)
}
func bindPointToMoveWidgets(p *coordinates.Point) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})
	ats := ui.GetUi().ActionTabs

	m := actions.NewMove(*p)

	// ats.BoundMove = nil
	// ats.BoundPoint = nil
	ats.BoundMove = binding.BindStruct(m)
	ats.BoundPoint = binding.BindStruct(p)
	x, _ := ats.BoundPoint.GetItem("X")
	y, _ := ats.BoundPoint.GetItem("Y")

	// ats.BoundMoveXSlider.Unbind()
	// ats.BoundMoveYSlider.Unbind()
	// ats.BoundMoveXEntry.Unbind()
	// ats.BoundMoveYEntry.Unbind()
	ats.BoundMoveXSlider.Bind(binding.IntToFloat(x.(binding.Int)))
	ats.BoundMoveYSlider.Bind(binding.IntToFloat(y.(binding.Int)))
	ats.BoundMoveXEntry.Bind(binding.IntToString(x.(binding.Int)))
	ats.BoundMoveYEntry.Bind(binding.IntToString(y.(binding.Int)))
	// ats.boundSpotSelect.Bind())

	x.AddListener(dl)
	y.AddListener(dl)
}

func bindAction(a actions.ActionInterface) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})
	ats := ui.GetUi().ActionTabs
	switch node := a.(type) {
	case *actions.Wait:
		ats.BoundWait = binding.BindStruct(node)
		t, _ := ats.BoundWait.GetItem("Time")

		ats.BoundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
		ats.BoundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

		t.AddListener(dl)
	case *actions.Move:
		ats.BoundMove = binding.BindStruct(node)
		ats.BoundPoint = binding.BindStruct(&node.Point)
		x, _ := ats.BoundPoint.GetItem("X")
		y, _ := ats.BoundPoint.GetItem("Y")

		ats.BoundMoveXSlider.Bind(binding.IntToFloat(x.(binding.Int)))
		ats.BoundMoveYSlider.Bind(binding.IntToFloat(y.(binding.Int)))
		ats.BoundMoveXEntry.Bind(binding.IntToString(x.(binding.Int)))
		ats.BoundMoveYEntry.Bind(binding.IntToString(y.(binding.Int)))
		// ats.boundSpotSelect.Bind())

		x.AddListener(dl)
		y.AddListener(dl)
	case *actions.Click:
		ats.BoundClick = binding.BindStruct(node)
		b, _ := ats.BoundClick.GetItem("Button")

		ats.BoundButtonToggle.Bind(custom_widgets.CustomStringToBool(b.(binding.String), "click", dl))

		b.AddListener(dl)
	case *actions.Key:
		ats.BoundKey = binding.BindStruct(node)
		k, _ := ats.BoundKey.GetItem("Key")
		s, _ := ats.BoundKey.GetItem("State")

		ats.BoundKeySelect.Bind(k.(binding.String))
		ats.BoundStateToggle.Bind(custom_widgets.CustomStringToBool(s.(binding.String), "key", dl))

		k.AddListener(dl)
		s.AddListener(dl)

	case *actions.Loop:
		ats.BoundLoop = binding.BindStruct(node)
		ats.BoundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		c, _ := ats.BoundLoop.GetItem("Count")
		n, _ := ats.BoundAdvancedAction.GetItem("Name")

		ats.BoundLoopNameEntry.Bind(n.(binding.String))
		ats.BoundCountLabel.Bind(binding.IntToString(c.(binding.Int)))
		ats.BoundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))

		c.AddListener(dl)
		n.AddListener(dl)
	case *actions.ImageSearch:
		ats.BoundImageSearch = binding.BindStruct(node)
		ats.BoundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		ats.BoundSearchArea = binding.BindStruct(&node.SearchArea)

		n, _ := ats.BoundAdvancedAction.GetItem("Name")
		sa, _ := ats.BoundSearchArea.GetItem("Name")
		t, _ := ats.BoundImageSearch.GetItem("Targets")

		ats.BoundImageSearchNameEntry.Bind(n.(binding.String))
		// ats.boundImageSearchAreaList.Select(slices.Index(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), node.SearchArea.Name))
		v, _ := ats.BoundImageSearchSearchAreaStringList.Get()
		for i, s := range v {
			if s == node.SearchArea.Name {
				ats.BoundImageSearchAreaList.Select(i)
			}
		}
		// ats.boundImageSearchAreaSelect.Bind(sa.(binding.String))
		ats.BoundImageSearch.SetValue("Targets", slices.Clone(node.Targets))

		t.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	case *actions.Ocr:
		ats.BoundOcr = binding.BindStruct(node)
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
