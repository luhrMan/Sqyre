package editor

import (
	"Sqyre/internal/capture"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"Sqyre/internal/validation"
	"Sqyre/ui/custom_widgets"
	"fmt"
	"image"
	_ "image/png"
	"os"
	"path/filepath"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

func setCollectionWidgets(c models.Collection, programName string) {
	ctw := shell().EditorTabs.CollectionsTab.Widgets
	ctw["Name"].(*widget.Entry).SetText(c.Name)
	refreshSearchAreaSelectOptions(ctw, programName)
	if sel, ok := ctw["searchAreaSelect"].(*widget.Select); ok {
		sel.SetSelected(c.SearchArea)
	}
	custom_widgets.SetEntryText(ctw["Rows"], strconv.Itoa(c.Rows))
	custom_widgets.SetEntryText(ctw["Cols"], strconv.Itoa(c.Cols))
	updateCollectionGridPreview(programName, c.Name, c.Rows, c.Cols)
	markCollectionsClean()
}

func collectionsAccordionConfig() entityAccordionConfig {
	tab := shell().EditorTabs.CollectionsTab
	return entityAccordionConfig{
		tab: tab,
		getKeys: func(p *models.Program) []string {
			return ProgramCollectionRepo(p).GetAllKeys()
		},
		sortKeys: sortCollectionKeysByDisplayName,
		getEntity: func(p *models.Program, key string) (string, error) {
			c, err := ProgramCollectionRepo(p).Get(key)
			if err != nil {
				return "", err
			}
			return c.Name, nil
		},
		onSelected: func(p *models.Program, key string) {
			c, err := ProgramCollectionRepo(p).Get(key)
			if err != nil {
				return
			}
			tab.SelectedItem = c
			tab.ProgramSelector.SetSelected(p.Name)
			setCollectionWidgets(*c, p.Name)
		},
	}
}

func setAccordionCollectionsLists(acc *custom_widgets.AccordionWithHeaderWidgets) {
	populateProgramEntityAccordion(acc, collectionsAccordionConfig())
}

func refreshCollectionsAccordionForProgram(programName string) {
	if acc, ok := shell().EditorTabs.CollectionsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		refreshProgramEntityAccordionRow(acc, collectionsAccordionConfig(), programName)
	}
}

func searchAreaOptionsForProgram(programName string) []string {
	pro, ok := getProgramForEditor(programName)
	if !ok {
		return nil
	}
	keys := ProgramSearchAreaRepo(pro, config.MainMonitorSizeString).GetAllKeys()
	sortSearchAreaKeysByDisplayName(pro, keys)
	// Prefer display names from entities
	out := make([]string, 0, len(keys))
	repo := ProgramSearchAreaRepo(pro, config.MainMonitorSizeString)
	for _, k := range keys {
		if sa, err := repo.Get(k); err == nil && sa != nil {
			out = append(out, sa.Name)
		} else {
			out = append(out, k)
		}
	}
	return out
}

func refreshSearchAreaSelectOptions(w map[string]fyne.CanvasObject, programName string) {
	sel, ok := w["searchAreaSelect"].(*widget.Select)
	if !ok {
		return
	}
	prev := sel.Selected
	sel.Options = searchAreaOptionsForProgram(programName)
	sel.Refresh()
	if prev != "" {
		for _, opt := range sel.Options {
			if opt == prev {
				sel.SetSelected(prev)
				return
			}
		}
	}
	if len(sel.Options) > 0 && sel.Selected == "" {
		sel.SetSelected(sel.Options[0])
	}
}

func collectionFromWidgets(w map[string]fyne.CanvasObject) *models.Collection {
	rows, _ := strconv.Atoi(custom_widgets.EntryText(w["Rows"]))
	cols, _ := strconv.Atoi(custom_widgets.EntryText(w["Cols"]))
	if rows < 1 {
		rows = 1
	}
	if cols < 1 {
		cols = 1
	}
	sa := ""
	if sel, ok := w["searchAreaSelect"].(*widget.Select); ok {
		sa = sel.Selected
	}
	return &models.Collection{
		Name:       w["Name"].(*widget.Entry).Text,
		SearchArea: sa,
		Rows:       rows,
		Cols:       cols,
	}
}

func validateCollectionForSave(w map[string]fyne.CanvasObject) error {
	c := collectionFromWidgets(w)
	return validation.ValidateCollectionFields(
		c.SearchArea,
		strconv.Itoa(c.Rows),
		strconv.Itoa(c.Cols),
	)
}

func collectionGridFromWidgets(w map[string]fyne.CanvasObject) *custom_widgets.CollectionGridView {
	if g, ok := w["collectionGrid"].(*custom_widgets.CollectionGridView); ok {
		return g
	}
	return nil
}

func updateCollectionGridPreview(programName, collectionName string, rows, cols int) {
	tab := shell().EditorTabs.CollectionsTab
	grid := collectionGridFromWidgets(tab.Widgets)
	if grid == nil {
		return
	}
	grid.SetGrid(rows, cols)
	path := config.CollectionImagePath(programName, collectionName)
	img, err := loadCollectionImage(path)
	if err != nil {
		grid.SetImage(nil)
		return
	}
	grid.SetImage(img)
}

func loadCollectionImage(path string) (image.Image, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	img, _, err := image.Decode(f)
	return img, err
}

func renameCollectionImage(programName, oldName, newName string) {
	if oldName == newName || oldName == "" || newName == "" {
		return
	}
	oldPath := config.CollectionImagePath(programName, oldName)
	newPath := config.CollectionImagePath(programName, newName)
	if err := os.Rename(oldPath, newPath); err != nil && !os.IsNotExist(err) {
		editorRepoLog("rename file", "collection image", oldPath, err)
	}
}

// captureAndSaveCollectionImage captures the linked search area and writes the static PNG.
func captureAndSaveCollectionImage(programName string, c *models.Collection) error {
	if c == nil || c.SearchArea == "" {
		return fmt.Errorf("collection has no search area")
	}
	program, ok := getProgramForEditor(programName)
	if !ok {
		return fmt.Errorf("program %q not found", programName)
	}
	sa, err := ProgramSearchAreaRepo(program, config.MainMonitorSizeString).Get(c.SearchArea)
	if err != nil {
		return fmt.Errorf("search area %q: %w", c.SearchArea, err)
	}
	b := searchAreaBoundsFrom(sa)
	b, err = resolveSearchAreaBounds("collection capture", sa, b)
	if err != nil {
		return err
	}
	img, _, _, _, _, err := capture.CaptureSearchArea(b.lx, b.ty, b.rx, b.by)
	if err != nil {
		return fmt.Errorf("capture: %w", err)
	}
	dir := filepath.Join(config.GetCollectionsPath(), programName)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create collections dir: %w", err)
	}
	path := config.CollectionImagePath(programName, c.Name)
	if err := screen.SavePNG(img, path); err != nil {
		return fmt.Errorf("save collection image: %w", err)
	}
	return nil
}

func setCollectionsButtons() {
	et := shell().EditorTabs
	tab := et.CollectionsTab
	if replaceBtn, ok := tab.Widgets["replaceButton"].(*widget.Button); ok {
		replaceBtn.OnTapped = func() {
			c, ok := tab.SelectedItem.(*models.Collection)
			if !ok || c == nil || c.Name == "" {
				editorErr(fmt.Errorf("select a collection first"))
				return
			}
			// Use current form values for search area / name
			cur := collectionFromWidgets(tab.Widgets)
			cur.Name = c.Name
			programName := tab.ProgramSelector.Selected
			if err := captureAndSaveCollectionImage(programName, cur); err != nil {
				editorErr(err)
				return
			}
			updateCollectionGridPreview(programName, cur.Name, cur.Rows, cur.Cols)
		}
	}
	if tab.ProgramSelector != nil {
		prev := tab.ProgramSelector.OnChanged
		tab.ProgramSelector.OnChanged = func(s string) {
			if prev != nil {
				prev(s)
			}
			refreshSearchAreaSelectOptions(tab.Widgets, s)
		}
	}
	// Keep grid overlay in sync with Rows/Cols edits
	for _, key := range []string{"Rows", "Cols"} {
		w := tab.Widgets[key]
		switch e := w.(type) {
		case *widget.Entry:
			prev := e.OnChanged
			e.OnChanged = func(s string) {
				if prev != nil {
					prev(s)
				}
				refreshCollectionGridFromForm()
			}
		case *custom_widgets.Incrementer:
			prev := e.OnChanged
			e.OnChanged = func(v int) {
				if prev != nil {
					prev(v)
				}
				refreshCollectionGridFromForm()
			}
		}
	}
}

func refreshCollectionGridFromForm() {
	tab := shell().EditorTabs.CollectionsTab
	c := collectionFromWidgets(tab.Widgets)
	programName := tab.ProgramSelector.Selected
	name := c.Name
	if v, ok := tab.SelectedItem.(*models.Collection); ok && v != nil && v.Name != "" {
		name = v.Name
	}
	updateCollectionGridPreview(programName, name, c.Rows, c.Cols)
}
