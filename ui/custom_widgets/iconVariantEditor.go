package custom_widgets

import (
	"Squire/internal/services"
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/storage"
	"fyne.io/fyne/v2/widget"
)

// IconVariantEditor is a custom Fyne widget for managing icon variants in the item editor panel
type IconVariantEditor struct {
	widget.BaseWidget

	programName     string
	itemName        string
	variants        []string
	service         *services.IconVariantService
	onVariantChange func()

	// UI components
	variantList     *fyne.Container
	addButton       *widget.Button
	mainContent     *fyne.Container
	window          fyne.Window
	thumbnailWidgets map[string]*IconThumbnail // Cache widgets to avoid recreation
}

// NewIconVariantEditor creates a new icon variant editor widget
func NewIconVariantEditor(programName, itemName string, service *services.IconVariantService, window fyne.Window, onVariantChange func()) *IconVariantEditor {
	editor := &IconVariantEditor{
		programName:      programName,
		itemName:         itemName,
		service:          service,
		window:           window,
		onVariantChange:  onVariantChange,
		thumbnailWidgets: make(map[string]*IconThumbnail),
	}

	editor.ExtendBaseWidget(editor)
	editor.loadVariants()
	editor.createUI()

	return editor
}

// loadVariants loads the list of variants from the filesystem
func (e *IconVariantEditor) loadVariants() {
	variants, err := e.service.GetVariants(e.programName, e.itemName)
	if err != nil {
		// Log error but continue with empty list
		fmt.Printf("Error loading variants: %v\n", err)
		e.variants = []string{}
		return
	}
	e.variants = variants
}

// createUI initializes the UI components
func (e *IconVariantEditor) createUI() {
	// Create the variant list container
	e.variantList = e.createVariantList()

	// Create "Add Icon Variant" button
	e.addButton = widget.NewButton("Add Icon Variant", func() {
		e.showAddVariantDialog()
	})

	// Layout: variant grid on top, add button at bottom
	e.mainContent = container.NewBorder(
		nil,
		e.addButton,
		nil,
		nil,
		container.NewVScroll(e.variantList),
	)
}

// createVariantList creates a grid of IconThumbnail widgets for existing variants
// Reuses existing widgets when possible to avoid recreation overhead
func (e *IconVariantEditor) createVariantList() *fyne.Container {
	if len(e.variants) == 0 {
		// Show a message when no variants exist
		label := widget.NewLabel("No icon variants found")
		return container.NewCenter(label)
	}

	// Create a grid container with thumbnails
	thumbnails := make([]fyne.CanvasObject, 0, len(e.variants))

	// Track which widgets are still needed
	neededWidgets := make(map[string]bool)

	for _, variantName := range e.variants {
		// Create a copy of variantName for the closure
		variant := variantName
		neededWidgets[variant] = true

		// Get the correct icon path for this item and variant
		iconPath := e.service.GetVariantPath(e.programName, e.itemName, variant)

		// Try to reuse existing widget
		thumbnail, exists := e.thumbnailWidgets[variant]
		if !exists {
			// Create new widget only if it doesn't exist
			// Create delete callback
			onDelete := func() {
				e.showDeleteConfirmation(variant)
			}

			thumbnail = NewIconThumbnail(iconPath, variant, onDelete)
			e.thumbnailWidgets[variant] = thumbnail
		} else {
			// Update existing widget with new icon path and callback
			thumbnail.SetIconPath(iconPath)
			thumbnail.SetOnDelete(func() {
				e.showDeleteConfirmation(variant)
			})
		}

		// Update delete button state
		if len(e.variants) <= 1 {
			thumbnail.deleteBtn.Disable()
		} else {
			thumbnail.deleteBtn.Enable()
		}

		thumbnails = append(thumbnails, thumbnail)
	}

	// Clean up widgets that are no longer needed
	for variant := range e.thumbnailWidgets {
		if !neededWidgets[variant] {
			delete(e.thumbnailWidgets, variant)
		}
	}

	return container.NewGridWrap(fyne.NewSize(100, 150), thumbnails...)
}

// showAddVariantDialog opens a file picker dialog to add a new icon variant
func (e *IconVariantEditor) showAddVariantDialog() {
	// Create file picker dialog
	fileDialog := dialog.NewFileOpen(func(reader fyne.URIReadCloser, err error) {
		if err != nil {
			dialog.ShowError(err, e.window)
			return
		}
		if reader == nil {
			// User cancelled
			return
		}
		defer reader.Close()

		sourcePath := reader.URI().Path()

		// Validate the file is a PNG
		if err := e.service.ValidateVariantFile(sourcePath); err != nil {
			dialog.ShowError(fmt.Errorf("Invalid PNG file: %v", err), e.window)
			return
		}

		// Prompt for variant name
		e.showVariantNameDialog(sourcePath)
	}, e.window)

	// Set file filter to PNG files
	fileDialog.SetFilter(storage.NewExtensionFileFilter([]string{".png"}))
	fileDialog.Show()
}

// showVariantNameDialog prompts the user to enter a variant name
func (e *IconVariantEditor) showVariantNameDialog(sourcePath string) {
	variantNameEntry := widget.NewEntry()
	variantNameEntry.SetPlaceHolder("Enter variant name (e.g., 'Ice', 'Fire', 'Original')")

	formDialog := dialog.NewForm("Add Icon Variant", "Add", "Cancel", []*widget.FormItem{
		widget.NewFormItem("Variant Name", variantNameEntry),
	}, func(confirmed bool) {
		if !confirmed {
			return
		}

		variantName := variantNameEntry.Text
		if variantName == "" {
			dialog.ShowError(fmt.Errorf("Variant name cannot be empty"), e.window)
			return
		}

		// Add the variant
		if err := e.service.AddVariant(e.programName, e.itemName, variantName, sourcePath); err != nil {
			dialog.ShowError(fmt.Errorf("Failed to add variant: %v", err), e.window)
			return
		}

		// Refresh the display
		e.refreshDisplay()
	}, e.window)

	formDialog.Resize(fyne.NewSize(400, 150))
	formDialog.Show()
}

// showDeleteConfirmation shows a confirmation dialog before deleting a variant
func (e *IconVariantEditor) showDeleteConfirmation(variantName string) {
	// Prevent deletion if only one variant remains
	if len(e.variants) <= 1 {
		dialog.ShowInformation("Cannot Delete", "Cannot delete the last icon variant. At least one variant must remain.", e.window)
		return
	}

	displayName := variantName
	if displayName == "" {
		displayName = "(default)"
	}

	confirmDialog := dialog.NewConfirm(
		"Delete Icon Variant",
		fmt.Sprintf("Are you sure you want to delete the variant '%s'?", displayName),
		func(confirmed bool) {
			if !confirmed {
				return
			}

			// Delete the variant
			if err := e.service.DeleteVariant(e.programName, e.itemName, variantName); err != nil {
				dialog.ShowError(fmt.Errorf("Failed to delete variant: %v", err), e.window)
				return
			}

			// Refresh the display
			e.refreshDisplay()
		},
		e.window,
	)

	confirmDialog.Show()
}

// refreshDisplay reloads variants and updates the UI
func (e *IconVariantEditor) refreshDisplay() {
	// Reload variants from filesystem
	e.loadVariants()

	// Recreate the variant list
	e.variantList = e.createVariantList()

	// Update the main content
	e.mainContent.Objects[0] = container.NewVScroll(e.variantList)
	e.mainContent.Refresh()

	// Call the variant change callback if provided
	if e.onVariantChange != nil {
		e.onVariantChange()
	}

	// Refresh the widget
	e.Refresh()
}

// CreateRenderer creates the widget renderer
func (e *IconVariantEditor) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(e.mainContent)
}

// SetProgramName updates the program name (call refreshDisplay manually after setting both)
func (e *IconVariantEditor) SetProgramName(programName string) {
	if e.programName != programName {
		e.programName = programName
		// Clear widget cache since program changed
		e.thumbnailWidgets = make(map[string]*IconThumbnail)
	}
}

// SetItemName updates the item name (call refreshDisplay manually after setting both)
func (e *IconVariantEditor) SetItemName(itemName string) {
	if e.itemName != itemName {
		e.itemName = itemName
		// Clear widget cache since item changed
		e.thumbnailWidgets = make(map[string]*IconThumbnail)
	}
}

// SetProgramAndItem updates both program and item name, then refreshes once
func (e *IconVariantEditor) SetProgramAndItem(programName, itemName string) {
	// Only refresh if values actually changed
	if e.programName == programName && e.itemName == itemName {
		return
	}
	
	// Clear widget cache since program or item changed
	e.thumbnailWidgets = make(map[string]*IconThumbnail)
	
	e.programName = programName
	e.itemName = itemName
	e.refreshDisplay()
}

// SetOnVariantChange updates the variant change callback
func (e *IconVariantEditor) SetOnVariantChange(callback func()) {
	e.onVariantChange = callback
}
