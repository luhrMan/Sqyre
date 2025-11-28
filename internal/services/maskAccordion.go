package services

import (
	"fmt"
	"log"
	"sort"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/storage"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

// MaskSelectionCallback is called when a mask is selected
type MaskSelectionCallback func(mask *MaskInfo)

// MaskAccordionService handles the population and management of the masks accordion
type MaskAccordionService struct {
	maskDiscovery     *MaskDiscoveryService
	selectionCallback MaskSelectionCallback
}

var maskAccordionServiceInstance *MaskAccordionService

// MaskAccordionServiceInstance returns the singleton instance of MaskAccordionService
func MaskAccordionServiceInstance() *MaskAccordionService {
	if maskAccordionServiceInstance == nil {
		maskAccordionServiceInstance = &MaskAccordionService{
			maskDiscovery: MaskDiscoveryServiceInstance(),
		}
	}
	return maskAccordionServiceInstance
}

// SetSelectionCallback sets the callback function to be called when a mask is selected
func (s *MaskAccordionService) SetSelectionCallback(callback MaskSelectionCallback) {
	s.selectionCallback = callback
}

// PopulateMasksAccordion populates the masks accordion with program-based mask lists
func (s *MaskAccordionService) PopulateMasksAccordion(accordion *widget.Accordion, widgetsMap map[string]fyne.Widget, previewImage *canvas.Image) error {
	// Clear existing accordion items
	accordion.Items = []*widget.AccordionItem{}

	// Get all masks organized by program with enhanced error handling
	masksByProgram, err := s.maskDiscovery.ScanMasksDirectory()
	if err != nil {
		log.Printf("Error scanning masks directory: %v", err)

		// Show error state in accordion
		s.showErrorState(accordion, "Failed to scan masks directory", err.Error())

		// Clear preview image
		s.ClearMaskPreview(previewImage)

		return fmt.Errorf("failed to populate masks accordion: %w", err)
	}

	// Handle case where no programs have masks
	if len(masksByProgram) == 0 {
		s.HandleEmptyPrograms(accordion)
		s.ClearMaskPreview(previewImage)
		return nil
	}

	// Get sorted program names for consistent display
	var programNames []string
	for programName := range masksByProgram {
		programNames = append(programNames, programName)
	}
	sort.Strings(programNames)

	// Create accordion items for each program with error recovery
	var creationErrors []string
	for _, programName := range programNames {
		masks := masksByProgram[programName]

		// Skip programs with no masks
		if len(masks) == 0 {
			continue
		}

		// Create accordion item with error handling
		accordionItem, err := s.createProgramMaskAccordionItemSafe(programName, masks, widgetsMap, previewImage)
		if err != nil {
			creationErrors = append(creationErrors, fmt.Sprintf("program '%s': %v", programName, err))
			continue
		}

		accordion.Append(accordionItem)
	}

	// Log creation errors for debugging
	if len(creationErrors) > 0 {
		for _, errMsg := range creationErrors {
			log.Printf("Warning: Failed to create accordion item for %s", errMsg)
		}
	}

	// If no accordion items were created successfully, show error state
	if len(accordion.Items) == 0 {
		s.showErrorState(accordion, "No masks could be loaded", "All programs failed to load due to errors")
		s.ClearMaskPreview(previewImage)
	}

	return nil
}

// createProgramMaskAccordionItem creates an accordion item for a specific program's masks
func (s *MaskAccordionService) createProgramMaskAccordionItem(programName string, masks []MaskInfo, widgetsMap map[string]fyne.Widget, previewImage *canvas.Image) *widget.AccordionItem {
	// Create the list structure for this program
	lists := struct {
		searchbar *widget.Entry
		maskList  *widget.List
		filtered  []MaskInfo
	}{
		searchbar: widget.NewEntry(),
		maskList:  widget.NewList(nil, nil, nil),
		filtered:  masks,
	}

	// Set up the search bar with error handling
	lists.searchbar.SetPlaceHolder("Search masks...")
	lists.searchbar.OnChanged = func(searchTerm string) {
		defer func() {
			if r := recover(); r != nil {
				log.Printf("Error: Panic during mask search in program '%s' with term '%s': %v",
					programName, searchTerm, r)
				// Reset to show all masks on error
				lists.filtered = masks
			}
		}()

		// Filter masks based on search term using fuzzy search for consistency with other tabs
		if searchTerm == "" {
			lists.filtered = masks
			log.Printf("Cleared search filter for program '%s', showing %d masks", programName, len(masks))
		} else {
			lists.filtered = []MaskInfo{}
			searchTerm = strings.ToLower(searchTerm)

			for _, mask := range masks {
				// Validate mask data before filtering
				if mask.Name == "" {
					log.Printf("Warning: Skipping mask with empty name in program '%s'", programName)
					continue
				}

				if fuzzy.MatchFold(searchTerm, mask.Name) {
					lists.filtered = append(lists.filtered, mask)
				}
			}

			log.Printf("Filtered masks in program '%s' with term '%s': %d results",
				programName, searchTerm, len(lists.filtered))
		}

		// Refresh list with error handling
		defer func() {
			if r := recover(); r != nil {
				log.Printf("Error: Panic while refreshing mask list for program '%s': %v", programName, r)
			}
		}()
		lists.maskList.Refresh()
	}

	// Set up the mask list
	lists.maskList = widget.NewList(
		func() int {
			return len(lists.filtered)
		},
		func() fyne.CanvasObject {
			return widget.NewLabel("Mask Name")
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= len(lists.filtered) {
				return
			}

			mask := lists.filtered[id]
			label := obj.(*widget.Label)
			label.SetText(mask.Name)
		},
	)

	// Handle deselection when clicking on empty areas
	lists.maskList.OnUnselected = func(id widget.ListItemID) {
		// Clear preview when item is unselected
		s.ClearMaskPreview(previewImage)
	}

	// Set up mask selection handler with error logging
	lists.maskList.OnSelected = func(id widget.ListItemID) {
		if id >= len(lists.filtered) {
			log.Printf("Error: Invalid mask selection ID %d (filtered list size: %d) in program: %s",
				id, len(lists.filtered), programName)
			return
		}

		selectedMask := lists.filtered[id]

		// Validate selected mask
		if selectedMask.Name == "" || selectedMask.Path == "" {
			log.Printf("Error: Invalid mask data - Name: '%s', Path: '%s', Program: '%s'",
				selectedMask.Name, selectedMask.Path, selectedMask.Program)
			s.showPreviewError(previewImage, "Invalid mask data")
			return
		}

		// Call the selection callback if set
		if s.selectionCallback != nil {
			defer func() {
				if r := recover(); r != nil {
					log.Printf("Error: Panic in mask selection callback for mask '%s': %v", selectedMask.Name, r)
				}
			}()
			s.selectionCallback(&selectedMask)
		}

		// Load and display the mask image in the preview area
		defer func() {
			if r := recover(); r != nil {
				log.Printf("Error: Panic while updating mask preview for '%s': %v", selectedMask.Name, r)
				s.showPreviewError(previewImage, "Preview update failed")
			}
		}()
		s.updateMaskPreview(selectedMask, previewImage)

		log.Printf("Successfully selected mask: %s from program: %s (path: %s)",
			selectedMask.Name, selectedMask.Program, selectedMask.Path)
	}

	// Store widgets in the widgets map for later access
	widgetsMap[programName+"-searchbar"] = lists.searchbar
	widgetsMap[programName+"-list"] = lists.maskList

	// Create and return the accordion item
	return widget.NewAccordionItem(
		programName,
		container.NewBorder(
			lists.searchbar,
			nil, nil, nil,
			lists.maskList,
		),
	)
}

// updateMaskPreview loads and displays the selected mask in the preview area
func (s *MaskAccordionService) updateMaskPreview(mask MaskInfo, previewImage *canvas.Image) {
	if previewImage == nil {
		log.Printf("Error: Preview image widget is nil for mask: %s", mask.Name)
		return
	}

	// Validate mask file before loading with detailed error reporting
	if err := s.maskDiscovery.ValidateMaskFile(mask.Path); err != nil {
		log.Printf("Error validating mask file %s: %v", mask.Path, err)

		// Determine specific error type for user-friendly message
		var errorMsg string
		if strings.Contains(err.Error(), "does not exist") {
			errorMsg = "Mask file not found"
		} else if strings.Contains(err.Error(), "permissions") {
			errorMsg = "Access denied to mask file"
		} else if strings.Contains(err.Error(), "too large") {
			errorMsg = "Mask file too large"
		} else if strings.Contains(err.Error(), "unsupported") {
			errorMsg = "Unsupported file format"
		} else {
			errorMsg = "Invalid mask file"
		}

		s.showPreviewError(previewImage, errorMsg)
		return
	}

	// Load the image from file with error recovery
	resource, err := storage.LoadResourceFromURI(storage.NewFileURI(mask.Path))
	if err != nil {
		log.Printf("Error loading mask image %s: %v", mask.Path, err)

		// Determine specific error type for user-friendly message
		var errorMsg string
		if strings.Contains(err.Error(), "permission") {
			errorMsg = "Access denied to mask file"
		} else if strings.Contains(err.Error(), "not found") || strings.Contains(err.Error(), "no such file") {
			errorMsg = "Mask file not found"
		} else {
			errorMsg = "Failed to load mask image"
		}

		s.showPreviewError(previewImage, errorMsg)
		return
	}

	// Validate that resource was loaded successfully
	if resource == nil {
		log.Printf("Error: Loaded resource is nil for mask: %s", mask.Path)
		s.showPreviewError(previewImage, "Failed to load mask resource")
		return
	}

	// Update the preview image with proportional scaling and aspect ratio maintenance
	previewImage.Resource = resource
	previewImage.Image = nil                        // Clear any existing image to force resource load
	previewImage.FillMode = canvas.ImageFillContain // Maintain aspect ratio and proportional scaling
	previewImage.Refresh()

	log.Printf("Successfully loaded mask preview: %s (%s) from %s", mask.Name, mask.Format, mask.Path)
}

// showPreviewError displays an error state in the preview area with comprehensive logging
func (s *MaskAccordionService) showPreviewError(previewImage *canvas.Image, errorMsg string) {
	if previewImage != nil {
		// Clear the image and show empty state
		previewImage.Resource = nil
		previewImage.Image = nil
		previewImage.FillMode = canvas.ImageFillContain
		previewImage.Refresh()

		log.Printf("Mask preview error: %s", errorMsg)
	} else {
		log.Printf("Error: Cannot show preview error - preview image widget is nil. Error was: %s", errorMsg)
	}
}

// ClearMaskPreview clears the mask preview area and shows placeholder state
func (s *MaskAccordionService) ClearMaskPreview(previewImage *canvas.Image) {
	if previewImage != nil {
		previewImage.Resource = nil
		previewImage.Image = nil
		previewImage.FillMode = canvas.ImageFillContain
		previewImage.Refresh()
	}

	// Call the selection callback with nil to clear selection
	if s.selectionCallback != nil {
		s.selectionCallback(nil)
	}
}

// InitializeMaskPreview sets up the initial placeholder state for the mask preview
func (s *MaskAccordionService) InitializeMaskPreview(previewImage *canvas.Image) {
	if previewImage != nil {
		// Set up initial state with proper scaling mode
		previewImage.FillMode = canvas.ImageFillContain
		previewImage.Resource = nil
		previewImage.Image = nil
		previewImage.Refresh()
	}
}

// HandleEmptyPrograms handles the case where no programs have masks
func (s *MaskAccordionService) HandleEmptyPrograms(accordion *widget.Accordion) {
	accordion.Items = []*widget.AccordionItem{}
}

// createProgramMaskAccordionItemSafe creates an accordion item with error handling
func (s *MaskAccordionService) createProgramMaskAccordionItemSafe(programName string, masks []MaskInfo, widgetsMap map[string]fyne.Widget, previewImage *canvas.Image) (*widget.AccordionItem, error) {
	// Validate inputs
	if programName == "" {
		return nil, fmt.Errorf("program name cannot be empty")
	}
	if widgetsMap == nil {
		return nil, fmt.Errorf("widgets map cannot be nil")
	}
	if len(masks) == 0 {
		return nil, fmt.Errorf("no masks provided for program")
	}

	// Create accordion item with error recovery
	defer func() {
		if r := recover(); r != nil {
			log.Printf("Panic recovered while creating accordion item for program '%s': %v", programName, r)
		}
	}()

	accordionItem := s.createProgramMaskAccordionItem(programName, masks, widgetsMap, previewImage)
	return accordionItem, nil
}

// showErrorState displays an error message in the accordion
func (s *MaskAccordionService) showErrorState(accordion *widget.Accordion, title, message string) {
	// Clear accordion
	accordion.Items = []*widget.AccordionItem{}

	// Create error label with wrapping
	errorLabel := widget.NewLabel(message)
	errorLabel.Wrapping = fyne.TextWrapWord

	// Create error item
	errorItem := widget.NewAccordionItem(
		title,
		container.NewCenter(errorLabel),
	)

	accordion.Append(errorItem)
	log.Printf("Showing error state in masks accordion: %s - %s", title, message)
}

// GetFilteredMasks returns filtered masks for a program based on search term
func (s *MaskAccordionService) GetFilteredMasks(programName, searchTerm string) ([]MaskInfo, error) {
	if programName == "" {
		return nil, fmt.Errorf("program name cannot be empty")
	}

	masks, err := s.maskDiscovery.GetMasksForProgram(programName)
	if err != nil {
		return nil, fmt.Errorf("failed to get masks for program '%s': %w", programName, err)
	}

	if searchTerm == "" {
		return masks, nil
	}

	// Filter masks using fuzzy search
	var filtered []MaskInfo
	searchTerm = strings.ToLower(searchTerm)

	for _, mask := range masks {
		if fuzzy.MatchFold(searchTerm, mask.Name) {
			filtered = append(filtered, mask)
		}
	}

	return filtered, nil
}
