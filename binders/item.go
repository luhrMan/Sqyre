package binders

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"image/color"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setItemsWidgets(i models.Item) {
	it := ui.GetUi().EditorTabs.ItemsTab.Widgets

	it["Name"].(*widget.Entry).SetText(i.Name)
	it["Cols"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[0]))
	it["Rows"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[1]))
	// it["Tags"].(*widget.Entry).Bind(c.(binding.String))
	it["StackMax"].(*widget.Entry).SetText(strconv.Itoa(i.StackMax))
}

func RefreshItemsAccordionItems() {
	for _, ai := range ui.GetUi().ActionTabs.ImageSearchItemsAccordion.Items {
		ai.Detail.Refresh()
	}
}

func setAccordionItemsLists(acc *widget.Accordion) {
	acc.Items = []*widget.AccordionItem{}

	var (
		ats   = ui.GetUi().ActionTabs
		icons = assets.BytesToFyneIcons()
	)
	for _, p := range repositories.ProgramRepo().GetAll() {
		lists := struct {
			searchbar *widget.Entry
			items     *widget.GridWrap
			filtered  []string
		}{
			searchbar: new(widget.Entry),
			items:     new(widget.GridWrap),
			filtered:  p.ItemRepo().GetAllKeys(),
		}
		lists.items = widget.NewGridWrap(
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

				stack := container.NewStack(rect, container.NewPadded(icon), ttwidget.NewLabel(""))
				return stack
			},
			func(id widget.GridWrapItemID, o fyne.CanvasObject) {
				name := lists.filtered[id]

				stack := o.(*fyne.Container)
				rect := stack.Objects[0].(*canvas.Rectangle)
				icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)
				tt := stack.Objects[2].(*ttwidget.Label)
				tt.SetToolTip(name)

				program, err := repositories.ProgramRepo().Get(p.Name)
				if err != nil {
					log.Printf("Error getting program %s: %v", p.Name, err)
					return
				}
				item, err := program.ItemRepo().Get(name)
				if err != nil {
					return
				}
				ist, _ := ats.BoundImageSearch.GetValue("Targets")
				t := ist.([]string)
				if ui.GetUi().MainUi.Visible() {
					if slices.Contains(t, p.Name+config.ProgramDelimiter+item.Name) {
						rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
					} else {
						rect.FillColor = color.RGBA{}
					}
				}

				path := p.Name + config.ProgramDelimiter + item.Name + config.PNG
				if icons[path] != nil {
					icon.Resource = icons[path]
				} else {
					icon.Resource = theme.BrokenImageIcon()
				}
				o.Refresh()
			},
		)
		lists.items.OnSelected = func(id widget.GridWrapItemID) {
			defer lists.items.RefreshItem(id)

			program, err := repositories.ProgramRepo().Get(p.Name)
			if err != nil {
				log.Printf("Error getting program %s: %v", p.Name, err)
				return
			}
			ui.GetUi().ProgramSelector.SetText(program.Name)
			itemName := lists.filtered[id]

			item, err := program.ItemRepo().Get(itemName)
			if err != nil {
				return
			}
			ui.GetUi().EditorTabs.ItemsTab.SelectedItem = item
			if ui.GetUi().MainUi.Visible() {
				if v, ok := ui.GetUi().Mui.MTabs.SelectedTab().Macro.Root.GetAction(ui.GetUi().Mui.MTabs.SelectedTab().SelectedNode).(*actions.ImageSearch); ok {
					t := v.Targets
					name := p.Name + config.ProgramDelimiter + item.Name
					if !slices.Contains(t, name) {
						t = append(t, name)
					} else {
						i := slices.Index(t, name)
						if i != -1 {
							t = slices.Delete(t, i, i+1)
						}
					}
					v.Targets = t
					ui.GetUi().Mui.MTabs.SelectedTab().Tree.RefreshItem(v.GetUID())
					// bindAction(v)

				}
				lists.items.UnselectAll()

			}
			setItemsWidgets(*item)
		}

		lists.searchbar = &widget.Entry{
			PlaceHolder: "Search here",
			OnChanged: func(s string) {
				defaultList := p.ItemRepo().GetAllKeys()
				defer lists.items.ScrollToTop()
				defer lists.items.Refresh()

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
			p.Name,
			container.NewBorder(
				lists.searchbar,
				nil, nil, nil,
				lists.items,
			),
		)
		ui.GetUi().EditorTabs.ItemsTab.Widgets[p.Name+"-searchbar"] = lists.searchbar
		ui.GetUi().EditorTabs.ItemsTab.Widgets[p.Name+"-list"] = lists.items

		acc.Append(&programItemsListWidget)
	}
}
