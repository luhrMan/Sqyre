package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"
	"fmt"
	"log"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

var (
	programFields     = []string{"Name"}
	itemFields        = []string{"Name", "Cols", "Rows", "StackMax"}
	pointFields       = []string{"Name", "X", "Y"}
	searchAreaFields  = []string{"Name", "LeftX", "TopY", "RightX", "BottomY"}
	maskFields        = []string{"Name", "shapeSelect", "CenterX", "CenterY", "Base", "Height", "Radius", "Inverse"}
	collectionFields  = []string{"Name", "searchAreaSelect", "Rows", "Cols"}
)

func getWidgetText(w fyne.CanvasObject) string {
	switch e := w.(type) {
	case *widget.Entry:
		return e.Text
	case *custom_widgets.VarEntry:
		return e.Text
	case *custom_widgets.VarEntryField:
		return e.Entry.Text
	case *widget.RadioGroup:
		return e.Selected
	case *widget.Select:
		return e.Selected
	case *custom_widgets.Incrementer:
		return strconv.Itoa(e.Value)
	case *widget.Check:
		if e.Checked {
			return "true"
		}
		return "false"
	}
	return ""
}

func markTabClean(tab *EditorTab, fields []string) {
	tab.OriginalValues = make(map[string]string)
	for _, f := range fields {
		tab.OriginalValues[f] = getWidgetText(tab.Widgets[f])
	}
	if tab.UpdateButton != nil {
		tab.UpdateButton.Disable()
	}
}

func checkTabDirty(tab *EditorTab, fields []string) {
	if tab.UpdateButton == nil || tab.OriginalValues == nil {
		return
	}
	dirty := false
	for _, f := range fields {
		if getWidgetText(tab.Widgets[f]) != tab.OriginalValues[f] {
			dirty = true
			break
		}
	}
	if dirty && allTabFieldsValid(tab) {
		tab.UpdateButton.Enable()
	} else {
		tab.UpdateButton.Disable()
	}
}

func setupDirtyTracking(tab *EditorTab, fields []string) {
	for _, f := range fields {
		w := tab.Widgets[f]
		switch e := w.(type) {
		case *widget.Entry:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *custom_widgets.VarEntry:
			prev := e.ChangedFn
			e.ChangedFn = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *custom_widgets.VarEntryField:
			prevChanged := e.OnChanged
			e.OnChanged = func(s string) {
				if prevChanged != nil {
					prevChanged(s)
				}
				checkTabDirty(tab, fields)
			}
			e.SetOnValidationChanged(func() {
				checkTabDirty(tab, fields)
			})
		case *widget.RadioGroup:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *widget.Check:
			prev := e.OnChanged
			e.OnChanged = func(checked bool) {
				if prev != nil {
					prev(checked)
				}
				checkTabDirty(tab, fields)
			}
		case *widget.Select:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				checkTabDirty(tab, fields)
			}
		case *custom_widgets.Incrementer:
			prev := e.OnChanged
			e.OnChanged = func(v int) {
				if prev != nil {
					prev(v)
				}
				checkTabDirty(tab, fields)
			}
		}
	}
}

func setupAllDirtyTracking() {
	et := shell().EditorTabs
	setupDirtyTracking(et.ProgramsTab, programFields)
	setupDirtyTracking(et.ItemsTab, itemFields)
	setupDirtyTracking(et.PointsTab, pointFields)
	setupDirtyTracking(et.SearchAreasTab, searchAreaFields)
	setupDirtyTracking(et.MasksTab, maskFields)
	setupDirtyTracking(et.CollectionsTab, collectionFields)
}

func markProgramsClean() {
	markTabClean(shell().EditorTabs.ProgramsTab, programFields)
}

func markItemsClean() {
	markTabClean(shell().EditorTabs.ItemsTab, itemFields)
}

func markPointsClean() {
	markTabClean(shell().EditorTabs.PointsTab, pointFields)
}

func markSearchAreasClean() {
	markTabClean(shell().EditorTabs.SearchAreasTab, searchAreaFields)
}

func markMasksClean() {
	markTabClean(shell().EditorTabs.MasksTab, maskFields)
}

func markCollectionsClean() {
	markTabClean(shell().EditorTabs.CollectionsTab, collectionFields)
}


// selectFirstProgramInEditorIfAny selects the first program (sorted keys) in the list and
// program selector when the editor UI is first wired up.
func selectFirstProgramInEditorIfAny() {
	if len(repositories.ProgramRepo().GetAllKeys()) == 0 {
		return
	}
	et := shell().EditorTabs
	if programList, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
		programList.Select(0)
	}
}

// updateProgramSelectorOptions refreshes every per-tab program selector with current programs.
func updateProgramSelectorOptions() {
	opts := repositories.ProgramRepo().GetAllKeys()
	et := shell().EditorTabs
	for _, tab := range []*EditorTab{
		et.ItemsTab, et.PointsTab,
		et.SearchAreasTab, et.MasksTab, et.CollectionsTab,
	} {
		if tab.ProgramSelector != nil {
			tab.ProgramSelector.Options = opts
			tab.ProgramSelector.Refresh()
		}
	}
}

func setEditorLists() {
	et := shell().EditorTabs
	setProgramList(
		et.ProgramsTab.Widgets["list"].(*widget.List),
	)
	setAccordionItemsLists(
		et.ItemsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	setAccordionPointsLists(
		et.PointsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	setAccordionSearchAreasLists(
		et.SearchAreasTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	setAccordionAutoPicSearchAreasLists(
		et.AutoPicTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	setAccordionMasksLists(
		et.MasksTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	setAccordionCollectionsLists(
		et.CollectionsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets),
	)
	et.ProgramsTab.SelectedItem = repositories.ProgramRepo().New()
	// Note: For nested models, we need a program context to get repositories
	// These will be set to proper instances when a program is selected
	et.ItemsTab.SelectedItem = &models.Item{}
	et.PointsTab.SelectedItem = &models.Point{}
	et.SearchAreasTab.SelectedItem = &models.SearchArea{}
	et.MasksTab.SelectedItem = &models.Mask{}
	et.CollectionsTab.SelectedItem = &models.Collection{}
	et.AutoPicTab.SelectedItem = &models.SearchArea{}
	shell().RefreshEditorActionBar()
}

func setEditorForms() {
	setEditorUpdateHandlers()
	setItemTagHandlers(shell().EditorTabs.ItemsTab)
}

func shouldConfirmOverwrite(entityType, targetName string, existsFn func(name string) bool, parent fyne.Window, onConfirm func()) bool {
	if !existsFn(targetName) {
		return false
	}
	dialogs.ShowConfirmWithEscape(
		"Confirm Overwrite",
		fmt.Sprintf("A %s named \"%s\" already exists. Overwrite it?", entityType, targetName),
		func(confirmed bool) {
			if !confirmed {
				return
			}
			onConfirm()
		},
		parent,
	)
	return true
}

// getOrCreateProgram retrieves a program by name or creates it if it doesn't exist.
func getOrCreateProgram(pn string) *models.Program {
	pro, err := repositories.ProgramRepo().Get(pn)
	if err != nil {
		pro = repositories.ProgramRepo().New()
		pro.Name = pn
		if err := repositories.ProgramRepo().Set(pro.Name, pro); err != nil {
			editorErr(err)
			return nil
		}
		log.Println("editor binder: new program created", pn)
		setEditorLists()
	}
	return pro
}

// getSelectedEntityName returns the display name of the currently selected entity on the active tab.
func getSelectedEntityName() string {
	et := shell().EditorTabs
	switch shell().EditorTabs.Selected().Text {
	case "Programs":
		if v, ok := et.ProgramsTab.SelectedItem.(*models.Program); ok {
			return v.Name
		}
	case "Items":
		if v, ok := et.ItemsTab.SelectedItem.(*models.Item); ok {
			return v.Name
		}
	case "Points":
		if v, ok := et.PointsTab.SelectedItem.(*models.Point); ok {
			return v.Name
		}
	case "Search Areas":
		if v, ok := et.SearchAreasTab.SelectedItem.(*models.SearchArea); ok {
			return v.Name
		}
	case "Masks":
		if v, ok := et.MasksTab.SelectedItem.(*models.Mask); ok {
			return v.Name
		}
	case "Collections":
		if v, ok := et.CollectionsTab.SelectedItem.(*models.Collection); ok {
			return v.Name
		}
	}
	return ""
}

// parseIntOrString attempts to parse s as an int; if it fails, returns s as-is.
func parseIntOrString(s string) any {
	if v, err := strconv.Atoi(s); err == nil {
		return v
	}
	return s
}

func setEditorButtons() {
	shell().AddButton.OnTapped = func() {
		var cfg createDialogConfig
		switch shell().EditorTabs.Selected().Text {
		case "Programs":
			cfg = programCreateConfig()
		case "Items":
			cfg = itemCreateConfig()
		case "Points":
			cfg = pointCreateConfig()
		case "Masks":
			cfg = maskCreateConfig()
		case "Collections":
			cfg = collectionCreateConfig()
		case "Search Areas":
			cfg = searchAreaCreateConfig()
		default:
			return
		}
		showCreateDialog(cfg, activeWire.Window)
	}
	shell().RemoveButton.OnTapped = func() {
		tabName := shell().EditorTabs.Selected().Text
		entityName := getSelectedEntityName()
		if entityName == "" {
			return
		}

		dialogs.ShowConfirmWithEscape(
			"Confirm Delete",
			fmt.Sprintf("Are you sure you want to delete %s \"%s\"?",
				strings.ToLower(tabName), entityName),
			func(confirmed bool) {
				if !confirmed {
					return
				}
				performDeleteForTab()
			},
			activeWire.Window,
		)
	}

}
