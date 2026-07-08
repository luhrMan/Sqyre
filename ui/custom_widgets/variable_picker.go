package custom_widgets

import (
	"Sqyre/internal/models"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// ShowVariablePicker opens a searchable popup below anchor listing variables.
func ShowVariablePicker(anchor fyne.CanvasObject, defs []models.VariableDef, onSelect func(name string)) {
	if len(defs) == 0 || onSelect == nil {
		return
	}
	holder := fyne.CurrentApp().Driver().CanvasForObject(anchor)
	if holder == nil {
		return
	}

	filter := NewFormEntry()
	filter.SetPlaceHolder("Filter variables…")

	labels := make([]string, len(defs))
	names := make([]string, len(defs))
	for i, d := range defs {
		names[i] = d.Name
		labels[i] = VariableDefLabel(d)
	}

	filtered := make([]int, len(defs))
	for i := range filtered {
		filtered[i] = i
	}

	var popup *widget.PopUp
	var list *widget.List

	list = widget.NewList(
		func() int { return len(filtered) },
		func() fyne.CanvasObject {
			return container.NewVBox(widget.NewLabel(""), widget.NewLabelWithStyle("", fyne.TextAlignLeading, fyne.TextStyle{Italic: true}))
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			box := obj.(*fyne.Container)
			nameLbl := box.Objects[0].(*widget.Label)
			metaLbl := box.Objects[1].(*widget.Label)
			idx := filtered[id]
			nameLbl.SetText(names[idx])
			meta := labels[idx]
			if meta == names[idx] {
				metaLbl.SetText("")
				metaLbl.Hide()
			} else {
				metaLbl.SetText(strings.TrimPrefix(meta, names[idx]+" · "))
				metaLbl.Show()
			}
		},
	)
	list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(filtered) {
			return
		}
		onSelect(names[filtered[id]])
		popup.Hide()
	}

	applyFilter := func(q string) {
		q = strings.ToLower(strings.TrimSpace(q))
		filtered = filtered[:0]
		for i, d := range defs {
			label := strings.ToLower(labels[i])
			if q == "" || strings.Contains(strings.ToLower(d.Name), q) || strings.Contains(label, q) {
				filtered = append(filtered, i)
			}
		}
		list.UnselectAll()
		RefreshListPreservingScroll(list)
	}

	filter.OnChanged = applyFilter
	filter.OnSubmitted = func(_ string) {
		if len(filtered) == 0 {
			return
		}
		onSelect(names[filtered[0]])
		popup.Hide()
	}

	content := container.NewBorder(filter, nil, nil, nil, list)
	popup = widget.NewPopUp(content, holder)

	width := anchor.Size().Width
	if width < 220 {
		width = 220
	}
	if contentW := variablePickerContentWidth(names, labels); contentW > width {
		width = contentW
	}
	if maxW := holder.Size().Width - theme.Padding()*2; maxW > 0 && width > maxW {
		width = maxW
	}
	itemH := float32(44)
	maxH := holder.Size().Height * 0.45
	listH := float32(len(filtered))*itemH + theme.Padding()*2
	if listH > maxH {
		listH = maxH
	}
	popup.Resize(fyne.NewSize(width, listH+filter.MinSize().Height+theme.Padding()*2))

	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(anchor)
	popup.ShowAtPosition(pos.Add(fyne.NewPos(0, anchor.Size().Height)))
	holder.Focus(filter)
}

// variablePickerContentWidth returns the width needed to display the widest
// list item (name or meta line) without horizontal truncation, including room
// for label insets and the list's scrollbar.
func variablePickerContentWidth(names, labels []string) float32 {
	textSize := theme.TextSize()
	var maxText float32
	measure := func(s string, style fyne.TextStyle) {
		if s == "" {
			return
		}
		if w := fyne.MeasureText(s, textSize, style).Width; w > maxText {
			maxText = w
		}
	}
	for i := range names {
		measure(names[i], fyne.TextStyle{})
		meta := strings.TrimPrefix(labels[i], names[i]+" · ")
		if meta != labels[i] {
			measure(meta, fyne.TextStyle{Italic: true})
		}
	}
	// Label inner padding (both sides) + list item inset + scrollbar clearance.
	return maxText + theme.Padding()*4 + theme.ScrollBarSize()
}
