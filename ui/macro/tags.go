package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/completionentry"
	"slices"
	"sort"
	"strings"
	"sync"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"

	kxlayout "github.com/ErikKalkoken/fyne-kx/layout"
)

const macroTagChipBtnSize float32 = 20

const (
	macroTagsPopupMinWidth        float32 = 320
	macroTagsPopupPreferredHeight float32 = 280
	macroTagsPopupMinScrollHeight float32 = 160
)

var macroTagsCache struct {
	mu    sync.Mutex
	tags  []string
	valid bool
}

// InvalidateMacroTagsCache clears the cached union of all macro tags.
func InvalidateMacroTagsCache() {
	macroTagsCache.mu.Lock()
	macroTagsCache.valid = false
	macroTagsCache.mu.Unlock()
}

// ResetMacroTagsCacheForTesting clears the macro tag cache (tests only).
func ResetMacroTagsCacheForTesting() {
	InvalidateMacroTagsCache()
}

func collectMacroTagsFromRepo() []string {
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
	macroTagsCache.mu.Lock()
	if macroTagsCache.valid {
		tags := macroTagsCache.tags
		macroTagsCache.mu.Unlock()
		return tags
	}
	macroTagsCache.mu.Unlock()

	tags := collectMacroTagsFromRepo()
	macroTagsCache.mu.Lock()
	macroTagsCache.tags = tags
	macroTagsCache.valid = true
	macroTagsCache.mu.Unlock()
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

func macroTagsMatchContainer(tagsContainer *fyne.Container, tags []string) bool {
	if tagsContainer == nil || len(tagsContainer.Objects) != len(tags) {
		return false
	}
	for i, tag := range tags {
		if macroTagChipLabelText(tagsContainer.Objects[i]) != tag {
			return false
		}
	}
	return true
}

func syncMacroTagsContainer(tagsContainer *fyne.Container, m *models.Macro, onRemove func(tag string)) {
	if tagsContainer == nil {
		return
	}
	var tags []string
	if m != nil {
		tags = m.Tags
	}
	if macroTagsMatchContainer(tagsContainer, tags) {
		return
	}
	existing := make(map[string]fyne.CanvasObject, len(tagsContainer.Objects))
	for _, obj := range tagsContainer.Objects {
		if label := macroTagChipLabelText(obj); label != "" {
			existing[label] = obj
		}
	}
	newObjects := make([]fyne.CanvasObject, 0, len(tags))
	for _, tag := range tags {
		if obj, ok := existing[tag]; ok {
			newObjects = append(newObjects, obj)
			delete(existing, tag)
			continue
		}
		tagToRemove := tag
		newObjects = append(newObjects, newMacroTagChip(tag, func() { onRemove(tagToRemove) }))
	}
	tagsContainer.Objects = newObjects
	tagsContainer.Refresh()
}

func appendMacroTagChip(tagsContainer *fyne.Container, tag string, onRemove func()) {
	if tagsContainer == nil {
		return
	}
	tagsContainer.Add(newMacroTagChip(tag, onRemove))
	tagsContainer.Refresh()
}

func removeMacroTagChip(tagsContainer *fyne.Container, tagToRemove string) bool {
	if tagsContainer == nil {
		return false
	}
	for i, obj := range tagsContainer.Objects {
		if macroTagChipLabelText(obj) == tagToRemove {
			tagsContainer.Objects = append(tagsContainer.Objects[:i], tagsContainer.Objects[i+1:]...)
			tagsContainer.Refresh()
			return true
		}
	}
	return false
}

func macroTagChipLabelText(chip fyne.CanvasObject) string {
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

func updateMacroTagsDisplay(mtabs *MacroTabs, m *models.Macro) {
	if mtabs == nil || mtabs.MacroTagsContainer == nil {
		return
	}
	syncMacroTagsContainer(mtabs.MacroTagsContainer, m, func(tag string) {
		removeMacroTag(mtabs, m, tag)
	})
	updateMacroTagsButton(mtabs, m)
	if (m == nil || len(m.Tags) == 0) && mtabs.macroTagsPopup != nil {
		mtabs.macroTagsPopup.Hide()
		mtabs.macroTagsPopup = nil
	}
}

func updateMacroTagsButton(mtabs *MacroTabs, m *models.Macro) {
	if mtabs == nil || mtabs.MacroTagsBtn == nil {
		return
	}
	btn := mtabs.MacroTagsBtn
	if m == nil || len(m.Tags) == 0 {
		btn.SetToolTip("No tags")
		btn.Disable()
		btn.Refresh()
		return
	}
	btn.SetToolTip(formatMacroTagsTooltip(m))
	btn.Enable()
	btn.Refresh()
}

func showMacroTagsPopup(mtabs *MacroTabs) {
	if mtabs == nil || mtabs.MacroTagsBtn == nil || mtabs.MacroTagsContainer == nil {
		return
	}
	mt := mtabs.SelectedTab()
	if mt == nil || mt.Macro == nil || len(mt.Macro.Tags) == 0 {
		return
	}
	anchor := mtabs.MacroTagsBtn
	holder := fyne.CurrentApp().Driver().CanvasForObject(anchor)
	if holder == nil {
		return
	}

	content := container.NewPadded(mtabs.MacroTagsContainer)
	scroll := container.NewScroll(content)
	popup := widget.NewPopUp(scroll, holder)
	mtabs.macroTagsPopup = popup

	popupSize, scrollSize := macroTagsPopupSize(holder.Size(), anchor, content.MinSize(), nil)
	scroll.Resize(scrollSize)
	popup.Resize(popupSize)

	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(anchor)
	popup.ShowAtPosition(pos.Add(fyne.NewPos(0, anchor.Size().Height)))
}

func macroTagsPopupSize(canvasSize fyne.Size, anchor fyne.CanvasObject, bodyMin fyne.Size, footer fyne.CanvasObject) (popupSize, scrollSize fyne.Size) {
	padding := theme.Padding() * 4
	footerH := float32(0)
	if footer != nil {
		footerH = footer.MinSize().Height
	}

	width := bodyMin.Width + padding
	if width < macroTagsPopupMinWidth {
		width = macroTagsPopupMinWidth
	}
	if anchor != nil {
		if anchorW := anchor.Size().Width * 3; anchorW > width {
			width = anchorW
		}
	}
	if maxW := canvasSize.Width - padding; maxW > 0 && width > maxW {
		width = maxW
	}

	maxH := canvasSize.Height * 0.55
	popupH := macroTagsPopupPreferredHeight
	if popupH > maxH {
		popupH = maxH
	}

	scrollH := popupH - footerH - padding
	if scrollH < macroTagsPopupMinScrollHeight {
		scrollH = macroTagsPopupMinScrollHeight
		popupH = scrollH + footerH + padding
		if popupH > maxH {
			popupH = maxH
			scrollH = popupH - footerH - padding
		}
	}
	if bodyMin.Height > scrollH {
		scrollH = bodyMin.Height
		popupH = scrollH + footerH + padding
		if popupH > maxH {
			popupH = maxH
			scrollH = popupH - footerH - padding
		}
	}

	innerW := width - padding
	if innerW < macroTagsPopupMinWidth-padding {
		innerW = macroTagsPopupMinWidth - padding
	}
	return fyne.NewSize(width, popupH), fyne.NewSize(innerW, scrollH)
}

func newMacroTagChip(tag string, onRemove func()) fyne.CanvasObject {
	tagLabel := widget.NewLabel(tag)
	tagLabel.Wrapping = fyne.TextWrapOff
	removeButton := widget.NewButtonWithIcon("", theme.CancelIcon(), onRemove)
	removeButton.Importance = widget.LowImportance
	chip := container.NewHBox(
		tagLabel,
		container.NewGridWrap(fyne.NewSize(macroTagChipBtnSize, macroTagChipBtnSize), removeButton),
	)
	return wrapMacroTagChip(chip)
}

func addMacroTag(m *models.Macro, tagText string) bool {
	if m == nil {
		return false
	}
	tagText = strings.TrimSpace(tagText)
	if tagText == "" {
		return false
	}
	if slices.Contains(m.Tags, tagText) {
		return false
	}
	m.Tags = append(m.Tags, tagText)
	if repositories.MacroRepo().Set(m.Name, m) != nil {
		return false
	}
	InvalidateMacroTagsCache()
	return true
}

func removeMacroTagFromMacro(m *models.Macro, tagToRemove string) bool {
	if m == nil {
		return false
	}
	newTags := make([]string, 0, len(m.Tags))
	for _, tag := range m.Tags {
		if tag != tagToRemove {
			newTags = append(newTags, tag)
		}
	}
	m.Tags = newTags
	if repositories.MacroRepo().Set(m.Name, m) != nil {
		return false
	}
	InvalidateMacroTagsCache()
	return true
}

func notifyMacroTagsChanged(m *models.Macro) {
	if m == nil || activeWire.Mui == nil || activeWire.Mui.MTabs == nil {
		return
	}
	mtabs := activeWire.Mui.MTabs
	if tree := mtabs.TreeForMacro(m.Name); tree != nil && tree.Macro != nil {
		tree.Macro.Tags = append([]string(nil), m.Tags...)
	}
	mt := mtabs.SelectedTab()
	if mt != nil && mt.Macro != nil && mt.Macro.Name == m.Name {
		mt.Macro.Tags = append([]string(nil), m.Tags...)
		updateMacroTagsDisplay(mtabs, m)
		refreshMacroTagEntryCompletion(mtabs, m)
	}
}

func removeMacroTag(mtabs *MacroTabs, m *models.Macro, tagToRemove string) {
	if !removeMacroTagFromMacro(m, tagToRemove) {
		return
	}
	if mtabs != nil && mtabs.MacroTagsContainer != nil {
		removeMacroTagChip(mtabs.MacroTagsContainer, tagToRemove)
	}
	updateMacroTagsButton(mtabs, m)
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
		tagText := mtabs.MacroTagEntry.Text
		mtabs.MacroTagEntry.HideCompletion()
		if !addMacroTag(m, tagText) && strings.TrimSpace(tagText) != "" {
			mtabs.MacroTagEntry.SetText("")
			return
		}
		mtabs.MacroTagEntry.SetText("")
		updateMacroTagsDisplay(mtabs, m)
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
	if mtabs.MacroTagsBtn != nil {
		mtabs.MacroTagsBtn.Importance = widget.LowImportance
		mtabs.MacroTagsBtn.OnTapped = func() { showMacroTagsPopup(mtabs) }
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

func formatMacroTagsTooltip(m *models.Macro) string {
	if m == nil || len(m.Tags) == 0 {
		return ""
	}
	return strings.Join(m.Tags, "\n")
}

func macroTagsListButtonTooltip(m *models.Macro) string {
	if tip := formatMacroTagsTooltip(m); tip != "" {
		return tip
	}
	return "Edit tags"
}

var activeMacroTagsEditorPopup *widget.PopUp

func hideMacroTagsEditorPopup() {
	if activeMacroTagsEditorPopup != nil {
		activeMacroTagsEditorPopup.Hide()
		activeMacroTagsEditorPopup = nil
	}
}

func showMacroTagsEditorPopup(anchor fyne.CanvasObject, m *models.Macro, onChanged func(*models.Macro)) {
	if anchor == nil || m == nil {
		return
	}
	holder := fyne.CurrentApp().Driver().CanvasForObject(anchor)
	if holder == nil {
		return
	}
	hideMacroTagsEditorPopup()

	tagsContainer := newMacroTagsContainer()
	tagEntry := completionentry.NewCompletionEntry([]string{})
	tagEntry.PlaceHolder = "Add tag…"
	tagSubmitBtn := widget.NewButtonWithIcon("", theme.ContentAddIcon(), nil)
	tagSubmitBtn.Importance = widget.MediumImportance

	refreshTagEntryCompletion := func() {
		currentText := tagEntry.Text
		if strings.TrimSpace(currentText) == "" {
			tagEntry.HideCompletion()
			return
		}
		matching := macroTagCompletionOptions(currentText, m, 10)
		tagEntry.SetOptions(matching)
		if len(matching) > 0 {
			tagEntry.ShowCompletion()
		} else {
			tagEntry.HideCompletion()
		}
	}

	var refreshTags func()
	refreshTags = func() {
		syncMacroTagsContainer(tagsContainer, m, func(tag string) {
			if removeMacroTagFromMacro(m, tag) {
				removeMacroTagChip(tagsContainer, tag)
				refreshTagEntryCompletion()
				notifyMacroTagsChanged(m)
				if onChanged != nil {
					onChanged(m)
				}
			}
		})
	}

	submitTag := func() {
		tagText := tagEntry.Text
		tagEntry.HideCompletion()
		if addMacroTag(m, tagText) {
			tagEntry.SetText("")
			appendMacroTagChip(tagsContainer, tagText, func() {
				if removeMacroTagFromMacro(m, tagText) {
					removeMacroTagChip(tagsContainer, tagText)
					refreshTagEntryCompletion()
					notifyMacroTagsChanged(m)
					if onChanged != nil {
						onChanged(m)
					}
				}
			})
			notifyMacroTagsChanged(m)
			if onChanged != nil {
				onChanged(m)
			}
			return
		}
		if strings.TrimSpace(tagText) != "" {
			tagEntry.SetText("")
		}
	}

	tagEntry.OnChanged = func(text string) {
		if strings.TrimSpace(text) == "" {
			tagEntry.HideCompletion()
			return
		}
		matching := macroTagCompletionOptions(text, m, 10)
		if len(matching) == 0 {
			tagEntry.HideCompletion()
			return
		}
		tagEntry.SetOptions(matching)
		tagEntry.ShowCompletion()
	}
	tagEntry.OnSubmitted = func(string) { submitTag() }
	tagSubmitBtn.OnTapped = submitTag

	refreshTags()

	entryRow := container.NewBorder(nil, nil, nil, tagSubmitBtn, tagEntry)
	scroll := container.NewScroll(tagsContainer)
	inner := container.NewBorder(nil, entryRow, nil, nil, scroll)
	content := container.NewPadded(inner)
	popup := widget.NewPopUp(content, holder)
	activeMacroTagsEditorPopup = popup

	popupSize, scrollSize := macroTagsPopupSize(holder.Size(), anchor, tagsContainer.MinSize(), entryRow)
	scroll.Resize(scrollSize)
	popup.Resize(popupSize)

	pos := fyne.CurrentApp().Driver().AbsolutePositionForObject(anchor)
	popup.ShowAtPosition(pos.Add(fyne.NewPos(0, anchor.Size().Height)))
}
