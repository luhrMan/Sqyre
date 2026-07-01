package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"sort"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
)

func newMacroTagsContainer() *fyne.Container {
	return container.New(kxlayout.NewRowWrapLayout())
}

func wrapMacroTagChip(inner fyne.CanvasObject) fyne.CanvasObject {
	if activeWire.WrapTagChip != nil {
		return activeWire.WrapTagChip(inner)
	}
	return inner
}

func getAllMacroTags() []string {
	tagMap := make(map[string]bool)
	for _, m := range repositories.MacroRepo().GetAll() {
		for _, tag := range m.Tags {
			tagMap[tag] = true
		}
	}
	tags := make([]string, 0, len(tagMap))
	for tag := range tagMap {
		tags = append(tags, tag)
	}
	sort.Strings(tags)
	return tags
}

func macroTagCompletionOptions(search string, macro *models.Macro, limit int) []string {
	tags := getAllMacroTags()
	if macro != nil {
		tags = excludeMacroTagsOnMacro(tags, macro.Tags)
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

func excludeMacroTagsOnMacro(tags []string, onMacro []string) []string {
	if len(onMacro) == 0 {
		return tags
	}
	filtered := make([]string, 0, len(tags))
	for _, tag := range tags {
		if !slices.Contains(onMacro, tag) {
			filtered = append(filtered, tag)
		}
	}
	return filtered
}

func updateMacroTagsDisplay(mtabs *MacroTabs, m *models.Macro) {
	if mtabs == nil || mtabs.MacroTagsContainer == nil {
		return
	}
	mtabs.MacroTagsContainer.Objects = nil
	if m == nil {
		mtabs.MacroTagsContainer.Refresh()
		return
	}
	for _, tag := range m.Tags {
		mtabs.MacroTagsContainer.Add(newMacroTagChip(mtabs, m, tag))
	}
	mtabs.MacroTagsContainer.Refresh()
}

func newMacroTagChip(mtabs *MacroTabs, m *models.Macro, tag string) fyne.CanvasObject {
	tagLabel := widget.NewLabel(tag)
	tagLabel.Importance = widget.MediumImportance
	tagToRemove := tag
	removeButton := widget.NewButtonWithIcon("", theme.DeleteIcon(), func() {
		removeMacroTag(mtabs, m, tagToRemove)
	})
	removeButton.Importance = widget.LowImportance
	return wrapMacroTagChip(container.NewPadded(container.NewHBox(tagLabel, removeButton)))
}

func removeMacroTag(mtabs *MacroTabs, m *models.Macro, tagToRemove string) {
	if m == nil {
		return
	}
	newTags := make([]string, 0, len(m.Tags))
	for _, tag := range m.Tags {
		if tag != tagToRemove {
			newTags = append(newTags, tag)
		}
	}
	m.Tags = newTags
	if err := repositories.MacroRepo().Set(m.Name, m); err != nil {
		return
	}
	updateMacroTagsDisplay(mtabs, m)
	refreshMacroTagEntryCompletion(mtabs, m)
}

func refreshMacroTagEntryCompletion(mtabs *MacroTabs, m *models.Macro) {
	if mtabs == nil || mtabs.MacroTagEntry == nil {
		return
	}
	currentText := mtabs.MacroTagEntry.Text
	if currentText == "" {
		return
	}
	matching := macroTagCompletionOptions(currentText, m, 10)
	mtabs.MacroTagEntry.SetOptions(matching)
	if len(matching) > 0 {
		mtabs.MacroTagEntry.ShowCompletion()
	} else {
		mtabs.MacroTagEntry.HideCompletion()
	}
}

func wireMacroTagHandlers(mtabs *MacroTabs) {
	if mtabs == nil || mtabs.MacroTagEntry == nil {
		return
	}

	submitTag := func() {
		mt := mtabs.SelectedTab()
		if mt == nil || mt.Macro == nil {
			return
		}
		m := mt.Macro
		tagText := strings.TrimSpace(mtabs.MacroTagEntry.Text)
		mtabs.MacroTagEntry.HideCompletion()
		if tagText == "" {
			return
		}
		for _, existing := range m.Tags {
			if existing == tagText {
				mtabs.MacroTagEntry.SetText("")
				return
			}
		}
		m.Tags = append(m.Tags, tagText)
		if err := repositories.MacroRepo().Set(m.Name, m); err != nil {
			return
		}
		mtabs.MacroTagEntry.SetText("")
		mtabs.MacroTagsContainer.Add(newMacroTagChip(mtabs, m, tagText))
		mtabs.MacroTagsContainer.Refresh()
	}

	mtabs.MacroTagEntry.OnChanged = func(text string) {
		mt := mtabs.SelectedTab()
		var m *models.Macro
		if mt != nil {
			m = mt.Macro
		}
		if strings.TrimSpace(text) == "" {
			mtabs.MacroTagEntry.HideCompletion()
			return
		}
		matching := macroTagCompletionOptions(text, m, 10)
		if len(matching) == 0 {
			mtabs.MacroTagEntry.HideCompletion()
			return
		}
		mtabs.MacroTagEntry.SetOptions(matching)
		mtabs.MacroTagEntry.ShowCompletion()
	}
	mtabs.MacroTagEntry.OnSubmitted = func(string) { submitTag() }
	if mtabs.MacroTagSubmitBtn != nil {
		mtabs.MacroTagSubmitBtn.OnTapped = submitTag
	}
}

func macroMatchesSearch(name, query string) bool {
	if fuzzy.MatchFold(query, name) {
		return true
	}
	m, err := repositories.MacroRepo().Get(name)
	if err != nil {
		return false
	}
	for _, tag := range m.Tags {
		if fuzzy.MatchFold(query, tag) {
			return true
		}
	}
	return false
}

func formatMacroListTags(m *models.Macro) string {
	if m == nil || len(m.Tags) == 0 {
		return ""
	}
	return strings.Join(m.Tags, " · ")
}
