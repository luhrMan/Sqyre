package actiondialog

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
	"Sqyre/internal/uiutil"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"image/color"
	"slices"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
	"github.com/go-vgo/robotgo"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func createImageSearchDialogContent(action *actions.ImageSearch) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetPlaceHolder("Display name for this action")
	nameEntry.SetText(action.Name)
	rowSplitEntry := widget.NewEntry()
	rowSplitEntry.SetPlaceHolder("e.g. 1")
	rowSplitEntry.SetText(fmt.Sprintf("%d", action.RowSplit))
	colSplitEntry := widget.NewEntry()
	colSplitEntry.SetPlaceHolder("e.g. 1")
	colSplitEntry.SetText(fmt.Sprintf("%d", action.ColSplit))
	toleranceMin, toleranceMax := 0.0, 1.0
	toleranceIncrementer := custom_widgets.NewFloatIncrementer(float64(action.Tolerance), 0.01, &toleranceMin, &toleranceMax, 2)
	toleranceIncrementer.SetValue(float64(action.Tolerance))
	blurMin, blurMax := 1, 21
	blurIncrementer := custom_widgets.NewIncrementer(action.Blur, 2, &blurMin, &blurMax)
	blurIncrementer.SetValue(action.Blur)
	outputXVarEntry := newVarEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputXVarEntry.SetPlaceHolder("e.g. foundX (sub-actions also get ${StackMax}, ${Cols}, ${Rows}, ${ItemName}, ${ImagePixelWidth}, ${ImagePixelHeight})")
	outputYVarEntry := newVarEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	outputYVarEntry.SetPlaceHolder("e.g. foundY")
	waitTil := newWaitTilFoundForm(action.WaitTilFound, action.WaitTilFoundSeconds, action.WaitTilFoundIntervalMs, 100)

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea
	tempTargets := slices.Clone(action.Targets)
	tempTargetsRef := &tempTargets

	// Search Areas accordion with searchbar above (fuzzy match program name + search area name)
	searchAreasSearchbar, searchAreasAccordion := buildSearchAreasAccordionWithSearchbar(func(sa actions.SearchArea) {
		tempSearchArea = sa
	})

	previewSize := fyne.NewSquareSize(30)
	var refreshItemsAccordion func()
	var removeTarget func(target string)

	previewList := widget.NewGridWrap(
		func() int { return len(tempTargets) },
		func() fyne.CanvasObject {
			icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
			icon.SetMinSize(previewSize)
			icon.FillMode = canvas.ImageFillContain
			removeBtn := ttwidget.NewButtonWithIcon("", theme.CancelIcon(), nil)
			removeBtn.Importance = widget.LowImportance
			return container.NewStack(icon, removeBtn)
		},
		func(id widget.GridWrapItemID, o fyne.CanvasObject) {
			if id >= len(tempTargets) {
				return
			}
			target := tempTargets[id]
			stack := o.(*fyne.Container)
			var newIcon *canvas.Image
			if path := uiutil.IconPathForTarget(target); path != "" {
				if res := assets.GetFyneResource(path); res != nil {
					newIcon = canvas.NewImageFromResource(res)
				} else {
					newIcon = canvas.NewImageFromResource(assets.AppIcon)
				}
			} else {
				newIcon = canvas.NewImageFromResource(assets.AppIcon)
			}
			newIcon.SetMinSize(previewSize)
			newIcon.FillMode = canvas.ImageFillContain
			stack.Objects[0] = newIcon

			removeBtn := stack.Objects[1].(*ttwidget.Button)
			removeBtn.OnTapped = func() {
				if removeTarget != nil {
					removeTarget(target)
				}
			}
			removeBtn.SetToolTip("Remove this program/item from the search targets (does not delete the item from your data).")
		},
	)
	previewLabel := ttwidget.NewLabel("Selected items:")
	previewLabel.SetToolTip("Icons for each item currently targeted. Sub-actions run for every match found, in order.")
	previewScroll := container.NewScroll(previewList)
	previewBox := container.NewBorder(
		previewLabel, nil, nil, nil,
		previewScroll,
	)
	previewBox.Hide() // show only when there are selected items

	refreshPreview := func() {
		if len(tempTargets) == 0 {
			previewBox.Hide()
		} else {
			previewBox.Show()
			previewList.Refresh()
		}
	}
	refreshPreview() // show initial selection

	removeTarget = func(target string) {
		t := *tempTargetsRef
		if i := slices.Index(t, target); i != -1 {
			t = slices.Delete(t, i, i+1)
			*tempTargetsRef = t
		}
		refreshPreview()
		if refreshItemsAccordion != nil {
			refreshItemsAccordion()
		}
	}

	// Items accordion with searchbar above (fuzzy match program name + item name/tags)
	var itemsSearchbar *widget.Entry
	var itemsAccordion fyne.CanvasObject
	itemsSearchbar, itemsAccordion, refreshItemsAccordion = buildItemsAccordionWithSearchbar(
		func() []string { return *tempTargetsRef },
		func(programName, baseItemName string) {
			name := programName + config.ProgramDelimiter + baseItemName
			t := *tempTargetsRef
			if i := slices.Index(t, name); i != -1 {
				t = slices.Delete(t, i, i+1)
			} else {
				t = append(t, name)
			}
			slices.Sort(t)
			*tempTargetsRef = t
			refreshPreview()
			refreshItemsAccordion()
		},
		func(newTargets []string) {
			*tempTargetsRef = newTargets
			refreshPreview()
			refreshItemsAccordion()
		},
		refreshPreview,
	)

	rightPanel := container.NewBorder(
		nil, nil,
		nil, nil,
		container.NewVSplit(
			container.NewVBox(
				widget.NewForm(
					formHint("Name:", nameEntry, "Display name for this action in the macro tree."),
					formHint("Row Split:", rowSplitEntry, "Persisted with the action for compatibility; the current matcher does not use this field."),
					formHint("Col Split:", colSplitEntry, "Persisted with the action for compatibility; the current matcher does not use this field."),
					formHint("Tolerance:", toleranceIncrementer, "Template match confidence threshold (0–1). Higher = stricter match; lower finds more candidates but may add false positives."),
					formHint("Blur:", container.NewVBox(blurIncrementer, layout.NewSpacer()), "Gaussian blur kernel size applied before matching. Odd values; larger smooths noise but reduces sharp detail."),
					formHint("Output X Variable:", outputXVarEntry, "Macro variable to store the matched X coordinate (screen pixels). Sub-actions can use ${StackMax}, ${Cols}, ${Rows}, ${ItemName}, ${ImagePixelWidth}, ${ImagePixelHeight} from the matched item."),
					formHint("Output Y Variable:", outputYVarEntry, "Macro variable to store the matched Y coordinate (screen pixels)."),
					formHint("", waitTil.Check, ""),
					formHint("Timeout (seconds):", waitTil.SecondsIncrementer, "With wait-until-found: maximum time to keep retrying before giving up."),
					formHint("Search interval (ms):", waitTil.IntervalIncrementer, "Delay between attempts when wait-until-found is enabled (minimum 100 ms in this dialog)."),
				),
			),
			previewBox,
		),
	)

	content :=
		container.NewHSplit(
			widget.NewAccordion(
				widget.NewAccordionItem("Search Areas",
					container.NewBorder(
						searchAreasSearchbar, nil, nil, nil,
						searchAreasAccordion,
					),
				),
				widget.NewAccordionItem("Items",
					container.NewBorder(
						itemsSearchbar, nil, nil, nil,
						itemsAccordion,
					),
				),
			),
			rightPanel,
		)

	saveFunc := func() {
		action.Name = nameEntry.Text
		if rs, err := strconv.Atoi(rowSplitEntry.Text); err == nil {
			action.RowSplit = rs
		}
		if cs, err := strconv.Atoi(colSplitEntry.Text); err == nil {
			action.ColSplit = cs
		}
		action.Tolerance = float32(toleranceIncrementer.Value)
		action.Blur = blurIncrementer.Value
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		waitTil.writeTo(&action.WaitTilFound, &action.WaitTilFoundSeconds, &action.WaitTilFoundIntervalMs)
		// Apply temporary changes
		action.SearchArea = tempSearchArea
		action.Targets = tempTargets
		slices.Sort(action.Targets)
	}

	return content, saveFunc
}

func createOcrDialogContent(action *actions.Ocr) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetPlaceHolder("Display name for this action")
	nameEntry.SetText(action.Name)
	targetEntry := newVarEntry()
	targetEntry.SetText(action.Target)
	outputVarEntry := newVarEntry()
	outputVarEntry.SetText(action.OutputVariable)
	outputXVarEntry := newVarEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputXVarEntry.SetPlaceHolder("e.g. foundX")
	outputYVarEntry := newVarEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	outputYVarEntry.SetPlaceHolder("e.g. foundY")
	waitTil := newWaitTilFoundForm(action.WaitTilFound, action.WaitTilFoundSeconds, action.WaitTilFoundIntervalMs, 0)

	// Temporary storage for changes (only applied on save)
	tempSearchArea := action.SearchArea

	// Search Areas accordion with searchbar above (fuzzy match program name + search area name)
	searchAreasSearchbar, searchAreasAccordion := buildSearchAreasAccordionWithSearchbar(func(sa actions.SearchArea) {
		tempSearchArea = sa
	})

	form := widget.NewForm(
		formHint("Name:", nameEntry, "Display name for this OCR action in the macro tree."),
		formHint("Text Target:", targetEntry, "Text to look for in the captured area (literal or ${variable}). Matching is fuzzy across OCR words."),
		formHint("Output Variable:", outputVarEntry, "Variable to store the full recognized text (or matched fragment)."),
		formHint("Output X Variable:", outputXVarEntry, "Variable for the X coordinate of the match (center of matched text in screen pixels)."),
		formHint("Output Y Variable:", outputYVarEntry, "Variable for the Y coordinate of the match (center of matched text in screen pixels)."),
		formHint("", waitTil.Check, ""),
		formHint("Timeout (seconds):", waitTil.SecondsIncrementer, "With wait-until-found: maximum time to keep scanning before continuing."),
		formHint("Search interval (ms):", waitTil.IntervalIncrementer, "Delay between OCR attempts when wait-until-found is enabled."),
	)

	content := container.NewHSplit(
		widget.NewAccordion(
			widget.NewAccordionItem("Search Areas",
				container.NewBorder(
					searchAreasSearchbar, nil, nil, nil,
					searchAreasAccordion,
				),
			),
		),
		form,
	)

	saveFunc := func() {
		action.Name = nameEntry.Text
		action.Target = targetEntry.Text
		action.OutputVariable = outputVarEntry.Text
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		waitTil.writeTo(&action.WaitTilFound, &action.WaitTilFoundSeconds, &action.WaitTilFoundIntervalMs)
		action.SearchArea = tempSearchArea
	}

	return content, saveFunc
}

func createFindPixelDialogContent(action *actions.FindPixel) (fyne.CanvasObject, func()) {
	nameEntry := widget.NewEntry()
	nameEntry.SetText(action.Name)
	nameEntry.SetPlaceHolder("Optional name for this action")

	tempSearchArea := action.SearchArea

	// Search Areas accordion with searchbar above (fuzzy match program name + search area name)
	searchAreasSearchbar, searchAreasAccordion := buildSearchAreasAccordionWithSearchbar(func(sa actions.SearchArea) {
		tempSearchArea = sa
	})

	colorEntry := widget.NewEntry()
	colorEntry.SetText(action.TargetColor)
	colorEntry.SetPlaceHolder("Hex e.g. ffffff or #ffffff")

	swatch := canvas.NewRectangle(color.RGBA{128, 128, 128, 255})
	swatch.SetMinSize(fyne.NewSize(32, 32))
	swatch.StrokeWidth = 1
	swatch.StrokeColor = color.RGBA{R: 80, G: 80, B: 80, A: 255}
	updateSwatch := func() {
		if c, ok := uiutil.HexToColor(colorEntry.Text); ok {
			swatch.FillColor = c
		}
		swatch.Refresh()
	}
	updateSwatch()
	colorEntry.OnChanged = func(string) { updateSwatch() }

	dropperBtn := ttwidget.NewButtonWithIcon("", theme.MediaRecordIcon(), func() {
		if active.ShowRecordingOverlay == nil {
			return
		}
		var dismissOverlay func()
		dismissOverlay = active.ShowRecordingOverlay(
			nil,
			func(ev *desktop.MouseEvent) {
				fyne.DoAndWait(func() {
					switch ev.Button {
					case desktop.MouseButtonPrimary:
						x, y := robotgo.Location()
						hex := robotgo.GetPixelColor(x, y)
						hex = strings.TrimPrefix(strings.ToLower(hex), "#")
						if len(hex) == 8 {
							hex = hex[2:]
						}
						colorEntry.SetText(hex)
						updateSwatch()
						dismissOverlay()
					default:
						dismissOverlay()
					}
				})
			},
		)
	})
	dropperBtn.Importance = widget.DangerImportance
	dropperBtn.SetToolTip("Pick a Color\nLeft click anywhere to sample the pixel color\nRight click or Escape to cancel")

	colorRow := container.NewBorder(
		nil, nil,
		swatch, dropperBtn,
		colorEntry,
	)

	toleranceEntry := widget.NewEntry()
	toleranceEntry.SetText(fmt.Sprintf("%d", action.ColorTolerance))
	toleranceEntry.SetPlaceHolder("0 = exact match")
	toleranceSlider := ttwidget.NewSlider(0, 100)
	toleranceSlider.SetValue(float64(action.ColorTolerance))
	toleranceSlider.OnChanged = func(f float64) {
		toleranceEntry.SetText(fmt.Sprintf("%.0f", f))
	}
	toleranceEntry.OnChanged = func(s string) {
		if val, err := strconv.ParseFloat(strings.TrimSpace(s), 64); err == nil {
			if val < 0 {
				val = 0
			}
			if val > 100 {
				val = 100
			}
			toleranceSlider.SetValue(val)
		}
	}
	pctLbl := widget.NewLabel("%")
	toleranceRow := container.NewHBox(toleranceEntry, pctLbl, toleranceSlider)

	outputXVarEntry := newVarEntry()
	outputXVarEntry.SetText(action.OutputXVariable)
	outputXVarEntry.SetPlaceHolder("e.g. foundX")
	outputYVarEntry := newVarEntry()
	outputYVarEntry.SetText(action.OutputYVariable)
	outputYVarEntry.SetPlaceHolder("e.g. foundY")

	waitTil := newWaitTilFoundForm(action.WaitTilFound, action.WaitTilFoundSeconds, action.WaitTilFoundIntervalMs, 100)

	form := widget.NewForm(
		formHint("Name:", nameEntry, "Optional label for this action in the macro tree."),
		formHint("Target color:", colorRow, "RGB hex color to find (e.g. ffffff). Use the eyedropper to sample from the screen."),
		formHint("Color tolerance:", toleranceRow, "Allowed deviation from the target color. Use the slider or type a value."),
		formHint("Output X Variable:", outputXVarEntry, "Variable to set to the found pixel X (screen coordinates)."),
		formHint("Output Y Variable:", outputYVarEntry, "Variable to set to the found pixel Y (screen coordinates)."),
		formHint("", waitTil.Check, ""),
		formHint("Timeout (seconds):", waitTil.SecondsIncrementer, "With wait-until-found: maximum time to scan before continuing."),
		formHint("Search interval (ms):", waitTil.IntervalIncrementer, "Delay between pixel scans when wait-until-found is enabled."),
	)

	content := container.NewHSplit(
		widget.NewAccordion(
			widget.NewAccordionItem("Search Areas",
				container.NewBorder(
					searchAreasSearchbar, nil, nil, nil,
					searchAreasAccordion,
				),
			),
		),
		form,
	)

	saveFunc := func() {
		action.Name = strings.TrimSpace(nameEntry.Text)
		action.SearchArea = tempSearchArea
		action.TargetColor = strings.ToLower(strings.TrimPrefix(strings.TrimSpace(colorEntry.Text), "#"))
		if t, err := strconv.Atoi(strings.TrimSpace(toleranceEntry.Text)); err == nil {
			if t < 0 {
				t = 0
			}
			if t > 100 {
				t = 100
			}
			action.ColorTolerance = t
		}
		action.OutputXVariable = outputXVarEntry.Text
		action.OutputYVariable = outputYVarEntry.Text
		waitTil.writeTo(&action.WaitTilFound, &action.WaitTilFoundSeconds, &action.WaitTilFoundIntervalMs)
	}

	return content, saveFunc
}

func createFocusWindowDialogContent(action *actions.FocusWindow) (fyne.CanvasObject, func()) {
	windowEntry := widget.NewEntry()
	windowEntry.SetText(action.WindowTarget)
	windowEntry.SetPlaceHolder("Type to search or pick from list (e.g. chrome, code)")

	// Full list from API; filtered list is what the list widget shows
	allWindowNames := []string{}
	filteredNames := []string{}

	applyFilter := func() {
		q := strings.TrimSpace(strings.ToLower(windowEntry.Text))
		if q == "" {
			filteredNames = make([]string, len(allWindowNames))
			copy(filteredNames, allWindowNames)
		} else {
			filteredNames = filteredNames[:0]
			for _, name := range allWindowNames {
				if fuzzy.Match(q, strings.ToLower(name)) {
					filteredNames = append(filteredNames, name)
				}
			}
		}
	}

	windowList := widget.NewList(
		func() int { return len(filteredNames) },
		func() fyne.CanvasObject { return widget.NewLabel("") },
		func(id widget.ListItemID, co fyne.CanvasObject) {
			if id < len(filteredNames) {
				co.(*widget.Label).SetText(filteredNames[id])
			}
		},
	)
	windowList.OnSelected = func(id widget.ListItemID) {
		if id >= 0 && id < len(filteredNames) {
			windowEntry.SetText(filteredNames[id])
		}
	}

	refreshList := func() {
		applyFilter()
		windowList.Refresh()
	}

	windowEntry.OnChanged = func(string) { refreshList() }

	refreshBtn := ttwidget.NewButton("Refresh list", func() {
		names, err := services.ActiveWindowNames()
		if err != nil {
			allWindowNames = []string{fmt.Sprintf("(error: %v)", err)}
		} else {
			allWindowNames = names
		}
		refreshList()
	})
	refreshBtn.SetToolTip("Reload the list of top-level window titles from the OS.")
	// Load list on open
	services.GoSafe(func() {
		names, err := services.ActiveWindowNames()
		if err != nil {
			fyne.Do(func() {
				allWindowNames = []string{fmt.Sprintf("(error: %v)", err)}
				refreshList()
			})
			return
		}
		fyne.Do(func() {
			allWindowNames = names
			refreshList()
		})
	})

	listHdr := ttwidget.NewLabel("Active windows (list filters as you type):")
	listHdr.SetToolTip("Windows currently reported by the system. Typing in the field above filters this list; click a row to fill the field.")

	listCard := container.NewBorder(
		listHdr,
		refreshBtn,
		nil, nil,
		windowList,
	)
	listCard.Resize(fyne.NewSize(400, 200))

	content := container.NewBorder(
		widget.NewForm(
			formHint("Window to focus / search:", windowEntry, "Substring or title used to find and activate a window. Fuzzy-matches against the list below."),
		),
		nil, nil, nil,
		listCard,
	)

	saveFunc := func() {
		action.WindowTarget = strings.TrimSpace(windowEntry.Text)
	}

	return content, saveFunc
}
