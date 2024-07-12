package gui

import (
	"Dark-And-Darker/structs"
	"log"
	"sort"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// type Spot struct {
// 	Name string
// 	// Add other fields as needed
// }

type CustomSelect struct {
	widget.Select
	categories map[string][]structs.Spot
}

func NewCustomSelect(data map[string][]structs.Spot) *CustomSelect {
	cs := &CustomSelect{
		categories: data,
	}

	var categories []string
	for category := range data {
		categories = append(categories, category)
	}
	sort.Strings(categories)

	var selectOptions []string
	for _, category := range categories {
		categoryOption := strings.ToUpper(category) // Use uppercase for category headers
		selectOptions = append(selectOptions, categoryOption)

		for _, spot := range data[category] {
			selectOption := "  " + spot.Name // Indent items within categories
			selectOptions = append(selectOptions, selectOption)
		}
	}
	log.Println(selectOptions)
	cs.Select = widget.Select{
		Options: selectOptions,
		OnChanged: func(selected string) {
			selected = strings.TrimSpace(selected) // Remove indentation
			//var index int
			for _, option := range selectOptions {
				if strings.TrimSpace(option) == selected {
					//index = i
					break
				}
			}
			log.Println(selected)
			// if index != -1 && index < len(cs.flattenedItems) && cs.flattenedItems[index] != "" {
			// 	cs.onSelected(cs.flattenedItems[index])
			// }
		},
	}

	return cs
}

func (s *CustomSelect) CreateRenderer() fyne.WidgetRenderer {
	//c := container.NewBorder(nil, nil, nil, nil, &s.Select)
	return widget.NewSimpleRenderer(&s.Select)
}

// CLAUDE ATTEMPT 1
// type CustomSelect struct {
// 	widget.List
// 	categories map[string][]structs.Spot
// 	flattened  []string
// 	onSelected func(string)
// }

// func NewCustomSelect(data map[string][]structs.Spot, onSelected func(string)) *CustomSelect {
// 	cs := &CustomSelect{
// 		categories: data,
// 		onSelected: onSelected,
// 	}

// 	var categories []string
// 	for category := range data {
// 		categories = append(categories, category)
// 	}
// 	sort.Strings(categories)

// 	for _, category := range categories {
// 		cs.flattened = append(cs.flattened, "**"+category+"**")
// 		for _, spot := range data[category] {
// 			cs.flattened = append(cs.flattened, spot.Name)
// 		}
// 	}

// 	cs.List = widget.List{
// 		Length: func() int {
// 			return len(cs.flattened)
// 		},
// 		CreateItem: func() fyne.CanvasObject {
// 			return widget.NewLabel("placeholder")
// 		},
// 		UpdateItem: func(id widget.ListItemID, item fyne.CanvasObject) {
// 			label := item.(*widget.Label)
// 			text := cs.flattened[id]
// 			if strings.HasPrefix(text, "**") && strings.HasSuffix(text, "**") {
// 				label.TextStyle = fyne.TextStyle{Bold: true}
// 				label.SetText(strings.Trim(text, "*"))
// 			} else {
// 				label.TextStyle = fyne.TextStyle{}
// 				label.SetText("  " + text) // Indent non-category items
// 			}
// 		},
// 	}

// 	cs.OnSelected = func(id widget.ListItemID) {
// 		selected := cs.flattened[id]
// 		if !strings.HasPrefix(selected, "**") {
// 			cs.onSelected(selected)
// 		}
// 	}

// 	return cs
// }
