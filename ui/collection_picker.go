package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"
	"Sqyre/ui/dialogs"
	"Sqyre/ui/editor"
	"fmt"
	"image"
	_ "image/png"
	"os"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	fynetooltip "github.com/dweymouth/fyne-tooltip"
)

// collectionPickerKeyPrefix marks collection entries inside point/search-area picker lists.
const collectionPickerKeyPrefix = "collection:"

func collectionPickerKey(name string) string {
	return collectionPickerKeyPrefix + name
}

func parseCollectionPickerKey(key string) (name string, ok bool) {
	return strings.CutPrefix(key, collectionPickerKeyPrefix)
}

func loadCollectionPickerImage(programName, collectionName string) (image.Image, error) {
	path := config.CollectionImagePath(programName, collectionName)
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	img, _, err := image.Decode(f)
	return img, err
}

// LoadCollectionPreviewImage loads the static collection PNG for list hover tooltips.
func LoadCollectionPreviewImage(p *models.Program, key string) (custom_widgets.PreviewTooltipResult, error) {
	name := key
	if n, ok := parseCollectionPickerKey(key); ok {
		name = n
	}
	img, err := loadCollectionPickerImage(p.Name, name)
	if err != nil {
		return custom_widgets.PreviewTooltipResult{}, err
	}
	return custom_widgets.PreviewTooltipResult{Image: img}, nil
}

func appendCollectionPickerKeys(p *models.Program, keys []string) []string {
	repo := editor.ProgramCollectionRepo(p)
	if repo == nil {
		return keys
	}
	for _, k := range repo.GetAllKeys() {
		keys = append(keys, collectionPickerKey(k))
	}
	return keys
}

func collectionDisplayName(p *models.Program, key string) string {
	name, ok := parseCollectionPickerKey(key)
	if !ok {
		name = key
	}
	repo := editor.ProgramCollectionRepo(p)
	if repo != nil {
		if c, err := repo.Get(name); err == nil && c != nil {
			return c.Name + " (collection)"
		}
	}
	return name + " (collection)"
}

// ShowCollectionCellPicker opens a grid popup to select a cell range from a collection.
func ShowCollectionCellPicker(parent fyne.Window, programName, collectionName string, initial actions.CoordinateRef, onSelect func(actions.CoordinateRef), onClosed func()) {
	if parent == nil || onSelect == nil {
		return
	}
	program, err := repositories.ProgramRepo().Get(programName)
	if err != nil || program == nil {
		dialogs.ShowErrorWithEscape(fmt.Errorf("program %q not found", programName), parent)
		if onClosed != nil {
			onClosed()
		}
		return
	}
	repo := editor.ProgramCollectionRepo(program)
	if repo == nil {
		dialogs.ShowErrorWithEscape(fmt.Errorf("collection repository unavailable"), parent)
		if onClosed != nil {
			onClosed()
		}
		return
	}
	col, err := repo.Get(collectionName)
	if err != nil || col == nil {
		dialogs.ShowErrorWithEscape(fmt.Errorf("collection %q not found", collectionName), parent)
		if onClosed != nil {
			onClosed()
		}
		return
	}

	grid := custom_widgets.NewCollectionGridView()
	grid.SetSelectable(true)
	grid.SetGrid(col.Rows, col.Cols)
	if img, err := loadCollectionPickerImage(programName, col.Name); err == nil {
		grid.SetImage(img)
	}
	if initial.IsCollection() && initial.Program() == programName && initial.Name() == col.Name {
		if r1, c1, r2, c2, ok := initial.CellRange(); ok {
			grid.SetSelection(r1, c1, r2, c2)
		}
	}

	status := widget.NewLabel("Click or drag to select cell(s)")
	grid.OnSelectionChanged = func(r1, c1, r2, c2 int) {
		if r1 == r2 && c1 == c2 {
			status.SetText(fmt.Sprintf("Selected cell R%d C%d", r1, c1))
		} else {
			status.SetText(fmt.Sprintf("Selected R%d–%d × C%d–%d", r1, r2, c1, c2))
		}
	}

	titleLabel := widget.NewLabelWithStyle(
		fmt.Sprintf("Select cells — %s", col.Name),
		fyne.TextAlignLeading,
		fyne.TextStyle{Bold: true},
	)
	saveBtn := widget.NewButtonWithIcon("", theme.DocumentSaveIcon(), nil)
	cancelBtn := widget.NewButtonWithIcon("", theme.CancelIcon(), nil)
	saveBtn.Importance = widget.HighImportance
	header := container.NewHBox(titleLabel, layout.NewSpacer(), saveBtn, cancelBtn)
	body := container.NewBorder(nil, status, nil, nil, container.NewPadded(grid))
	content := container.NewBorder(header, nil, nil, nil, body)
	pop := widget.NewModalPopUp(wrapEntityPickerPopupContent(content), parent.Canvas())
	fynetooltip.AddPopUpToolTipLayer(pop)
	dlg := dialogs.AddPopupEscapeClose(pop, parent)
	if onClosed != nil {
		dlg.SetOnClosed(onClosed)
	}
	saveBtn.OnTapped = func() {
		r1, c1, r2, c2, ok := grid.Selection()
		if !ok {
			dialogs.ShowErrorWithEscape(fmt.Errorf("select at least one cell"), parent)
			return
		}
		onSelect(actions.NewCollectionRef(programName, col.Name, r1, c1, r2, c2))
		deferHidePickerDialog(dlg)
	}
	cancelBtn.OnTapped = func() { deferHidePickerDialog(dlg) }
	dlg.Resize(entityPickerSize(parent.Canvas()))
	dlg.Show()
}
