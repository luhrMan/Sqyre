package editor

import (
	"log"
	"slices"
	"strings"

	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/completionentry"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// updateTagsDisplay updates the tags grid container with the current item's tags
func updateTagsDisplay(item *models.Item) {
	it := shell().EditorTabs.ItemsTab.Widgets
	tagsContainer, ok := it["Tags"].(*fyne.Container)
	if !ok {
		return
	}

	// Clear existing tags
	tagsContainer.Objects = []fyne.CanvasObject{}

	// Add each tag as a label with a remove button
	for _, tag := range item.Tags {
		tagsContainer.Add(newTagChip(item, tag))
	}

	tagsContainer.Refresh()
}

// newTagChip builds a single tag chip (label + remove button) for item.
func newTagChip(item *models.Item, tag string) fyne.CanvasObject {
	tagLabel := widget.NewLabel(tag)
	removeButton := widget.NewButtonWithIcon("", theme.DeleteIcon(), func() {
		removeTag(item, tag)
	})
	removeButton.Importance = widget.LowImportance
	return wrapTagChip(container.NewHBox(tagLabel, removeButton))
}

// appendTagChip adds a single chip for tag to the Items tab tags container without
// rebuilding the existing chips (used on the hot "add tag" path).
func appendTagChip(item *models.Item, tag string) {
	it := shell().EditorTabs.ItemsTab.Widgets
	tagsContainer, ok := it["Tags"].(*fyne.Container)
	if !ok {
		return
	}
	tagsContainer.Add(newTagChip(item, tag))
	tagsContainer.Refresh()
}

// removeTagChip removes one tag chip from the Items tab without rebuilding the others.
func removeTagChip(tagsContainer *fyne.Container, tagToRemove string) bool {
	for i, obj := range tagsContainer.Objects {
		if tagChipLabelText(obj) == tagToRemove {
			tagsContainer.Objects = append(tagsContainer.Objects[:i], tagsContainer.Objects[i+1:]...)
			tagsContainer.Refresh()
			return true
		}
	}
	return false
}

// tagChipLabelText reads the tag label from a chip built by newTagChip / wrapTagChip.
func tagChipLabelText(chip fyne.CanvasObject) string {
	outer := chip
	if box, ok := outer.(*fyne.Container); ok && len(box.Objects) == 1 {
		outer = box.Objects[0]
	}
	row, ok := outer.(*fyne.Container)
	if !ok || len(row.Objects) == 0 {
		return ""
	}
	lbl, ok := row.Objects[0].(*widget.Label)
	if !ok {
		return ""
	}
	return lbl.Text
}

// removeTag removes a tag from the current item and saves it
func removeTag(item *models.Item, tagToRemove string) {
	// Remove the tag from the slice
	newTags := []string{}
	for _, tag := range item.Tags {
		if tag != tagToRemove {
			newTags = append(newTags, tag)
		}
	}
	item.Tags = newTags

	// Save the item
	p := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
	program, err := repositories.ProgramRepo().Get(p)
	if err != nil {
		log.Printf("Error getting program %s: %v", p, err)
		return
	}

	if err := ProgramItemRepo(program).Set(item.Name, item); err != nil {
		log.Printf("Error saving item %s: %v", item.Name, err)
		return
	}

	// Reload the item from the repository to ensure SelectedItem is in sync
	updatedItem, err := ProgramItemRepo(program).Get(item.Name)
	if err != nil {
		log.Printf("Error reloading item %s: %v", item.Name, err)
		if it := shell().EditorTabs.ItemsTab.Widgets; it != nil {
			if tagsContainer, ok := it["Tags"].(*fyne.Container); ok {
				removeTagChip(tagsContainer, tagToRemove)
			}
		}
		return
	}

	// Update the SelectedItem to the reloaded item
	shell().EditorTabs.ItemsTab.SelectedItem = updatedItem
	shell().RefreshEditorActionBar()

	InvalidateProgramTagsCache(p)
	if it := shell().EditorTabs.ItemsTab.Widgets; it != nil {
		if tagsContainer, ok := it["Tags"].(*fyne.Container); ok {
			removeTagChip(tagsContainer, tagToRemove)
		}
	}

	// Refresh the tag entry's completion options to ensure deleted tags are removed from suggestions
	it := shell().EditorTabs.ItemsTab.Widgets
	if tagEntry, ok := it["tagEntry"].(*completionentry.CompletionEntry); ok {
		currentText := tagEntry.Text
		// If there's text in the entry, refresh the completion options
		if currentText != "" {
			programName := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
			var item *models.Item
			if v, ok := shell().EditorTabs.ItemsTab.SelectedItem.(*models.Item); ok {
				item = v
			}
			matchingTags := tagCompletionOptions(programName, currentText, item, 10)
			tagEntry.SetOptions(matchingTags)
			if len(matchingTags) > 0 {
				tagEntry.ShowCompletion()
			} else {
				tagEntry.HideCompletion()
			}
		}
	}
}

// getProgramTags collects all unique tags from items in the given program (cached).
func getProgramTags(programName string) []string {
	programTagsCache.mu.Lock()
	if programTagsCache.byProgram == nil {
		programTagsCache.byProgram = make(map[string][]string)
	}
	if tags, ok := programTagsCache.byProgram[programName]; ok {
		programTagsCache.mu.Unlock()
		return tags
	}
	programTagsCache.mu.Unlock()

	tags := collectProgramTagsFromRepo(programName)
	programTagsCache.mu.Lock()
	programTagsCache.byProgram[programName] = tags
	programTagsCache.mu.Unlock()
	return tags
}

// tagCompletionOptions returns program tags matching search, excluding tags already on item.
// limit <= 0 means no limit.
func tagCompletionOptions(programName, search string, item *models.Item, limit int) []string {
	tags := getProgramTags(programName)
	if item != nil {
		tags = excludeTagsOnItem(tags, item.Tags)
	}
	if search == "" {
		if limit > 0 && len(tags) > limit {
			return tags[:limit]
		}
		return tags
	}

	searchLower := strings.ToLower(search)
	matching := make([]string, 0, len(tags))
	for _, tag := range tags {
		if strings.Contains(strings.ToLower(tag), searchLower) {
			matching = append(matching, tag)
		}
	}
	if limit > 0 && len(matching) > limit {
		return matching[:limit]
	}
	return matching
}

func excludeTagsOnItem(tags []string, onItem []string) []string {
	if len(onItem) == 0 {
		return tags
	}
	filtered := make([]string, 0, len(tags))
	for _, tag := range tags {
		if !slices.Contains(onItem, tag) {
			filtered = append(filtered, tag)
		}
	}
	return filtered
}
