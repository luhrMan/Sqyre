package binders

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"image/color"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func bindItemsWidgets(di binding.Struct, bx, by binding.Int) {
	dl := binding.NewDataListener(func() {
		mt := ui.GetUi().Mui.MTabs.SelectedTab()
		fyne.Do(func() { mt.RefreshItem(mt.SelectedNode) })
	})

	it := ui.GetUi().EditorTabs.ItemsTab.Widgets

	name, _ := di.GetItem("Name")
	// gsx, _ := bx.GetItem("X")
	// gsy, _ := gs.GetItem("Y")
	// c, _ := di.GetItem("Tags")
	sm, _ := di.GetItem("StackMax")
	m, _ := di.GetItem("Merchant")

	// it["Name"].(*widget.Entry).Unbind()
	// it["Rows"].(*widget.Entry).Unbind()
	// it["Cols"].(*widget.Entry).Unbind()
	// widget.NewCard("test card", "", nil)
	// it["Tags"].(*widget.Entry).Unbind()
	// it["StackMax"].(*widget.Entry).Unbind()
	// it["Merchant"].(*widget.Entry).Unbind()
	// gs.RemoveListener(dl)
	// c.RemoveListener(dl)
	// sm.RemoveListener(dl)
	// m.RemoveListener(dl)

	it["Name"].(*widget.Entry).Bind(name.(binding.String))
	it["GridSizeX"].(*widget.Entry).Bind(binding.IntToString(bx))
	it["GridSizeY"].(*widget.Entry).Bind(binding.IntToString(by))
	// it["Tags"].(*widget.Entry).Bind(c.(binding.String))
	it["StackMax"].(*widget.Entry).Bind(binding.IntToString(sm.(binding.Int)))
	it["Merchant"].(*widget.Entry).Bind(m.(binding.String))
	// gs.AddListener(dl)
	// c.AddListener(dl)
	sm.AddListener(dl)
	m.AddListener(dl)
}

func RefreshItemsAccordionItems() {
	for _, ai := range ui.GetUi().ActionTabs.ImageSearchItemsAccordion.Items {
		ai.Detail.Refresh()
	}
}

func setAccordionItemsLists(acc *widget.Accordion) {
	var (
		ats   = ui.GetUi().ActionTabs
		icons = assets.BytesToFyneIcons()
	)
	for _, pb := range GetBoundPrograms() {
		lists := struct {
			boundItemSearchBar *widget.Entry
			boundItemGrid      *widget.GridWrap
			filtered           []string
		}{
			boundItemSearchBar: &widget.Entry{},
			boundItemGrid:      &widget.GridWrap{},
			filtered:           pb.Program.GetItemsAsStringSlice(),
		}
		lists.boundItemGrid = widget.NewGridWrap(
			func() int {
				return len(lists.filtered)
			},
			func() fyne.CanvasObject {
				rect := canvas.NewRectangle(color.RGBA{})
				rect.SetMinSize(fyne.NewSquareSize(45))
				rect.CornerRadius = 5

				icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
				icon.SetMinSize(fyne.NewSquareSize(40))
				icon.FillMode = canvas.ImageFillOriginal

				stack := container.NewStack(rect, container.NewPadded(icon))
				return stack
			},
			func(id widget.GridWrapItemID, o fyne.CanvasObject) {
				item := lists.filtered[id]
				boundItem := pb.ItemBindings[item]
				name, _ := boundItem.GetValue("Name")

				stack := o.(*fyne.Container)
				rect := stack.Objects[0].(*canvas.Rectangle)
				icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)

				ist, _ := ats.BoundImageSearch.GetValue("Targets")
				t := ist.([]string)

				if slices.Contains(t, strings.ToLower(pb.Program.Name)+config.ProgramDelimiter+name.(string)) {
					rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
				} else {
					rect.FillColor = color.RGBA{}
				}

				path := pb.Program.Name + config.ProgramDelimiter + name.(string) + config.PNG
				if icons[path] != nil {
					icon.Resource = icons[path]
				} else {
					icon.Resource = theme.BrokenImageIcon()
				}
				o.Refresh()
			},
		)
		lists.boundItemGrid.OnSelected = func(id widget.GridWrapItemID) {
			defer lists.boundItemGrid.UnselectAll()
			defer lists.boundItemGrid.RefreshItem(id)
			// boundMacro := boundMacros[ui.GetUi().Mui.MTabs.SelectedTab().Macro.Name]

			item := lists.filtered[id]
			boundItem := pb.ItemBindings[item]
			i, _ := repositories.ProgramRepo().Get(pb.Program.Name).GetItem(item)
			boundx := binding.BindInt(&i.GridSize[0])
			boundy := binding.BindInt(&i.GridSize[1])
			bindItemsWidgets(boundItem, boundx, boundy)
			n, _ := boundItem.GetValue("Name")
			ist, _ := ats.BoundImageSearch.GetValue("Targets")
			t := ist.([]string)
			name := pb.Program.Name + config.ProgramDelimiter + n.(string)
			if !slices.Contains(t, name) {
				t = append(t, name)
			} else {
				i := slices.Index(t, name)
				if i != -1 {
					t = slices.Delete(t, i, i+1)
				}
			}
			ats.BoundImageSearch.SetValue("Targets", t)
			// for _, o := range ui.GetUi().EditorTabs.ItemsTab.Right.Objects {
			// 	o.Refresh()
			// }
			// boundMacro.bindAction(v)

		}

		lists.boundItemSearchBar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				// defaultList := pro.Coordinates[config.MainMonitorSizeString].Points
				defaultList := pb.Program.GetItemsAsStringSlice()
				defer lists.boundItemGrid.ScrollToTop()
				defer lists.boundItemGrid.Refresh()

				if s == "" {
					lists.filtered = defaultList
					return
				}
				lists.filtered = []string{}
				for _, i := range defaultList {
					if fuzzy.MatchFold(s, i) {
						lists.filtered = append(lists.filtered, i)
					}
				}
			},
		}
		programItemsListWidget := *widget.NewAccordionItem(
			pb.Program.Name,
			container.NewBorder(
				lists.boundItemSearchBar,
				nil, nil, nil,
				lists.boundItemGrid,
			),
		)
		acc.Append(&programItemsListWidget)
	}
}
