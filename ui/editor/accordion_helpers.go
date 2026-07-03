package editor

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"sort"
	"strings"

	"fyne.io/fyne/v2/widget"

	"github.com/lithammer/fuzzysearch/fuzzy"
)

// accordionProgramNameFromTitle extracts "MyProgram" from titles like "MyProgram (3)".
func accordionProgramNameFromTitle(title string) string {
	if i := strings.LastIndex(title, " ("); i > 0 {
		return title[:i]
	}
	return title
}

func captureAccordionOpenByProgram(items []*widget.AccordionItem) map[string]bool {
	open := make(map[string]bool, len(items))
	for _, item := range items {
		if item == nil {
			continue
		}
		open[accordionProgramNameFromTitle(item.Title)] = item.Open
	}
	return open
}

func applyAccordionOpenByProgram(items []*widget.AccordionItem, open map[string]bool) {
	for _, item := range items {
		if item == nil {
			continue
		}
		name := accordionProgramNameFromTitle(item.Title)
		if was, ok := open[name]; ok {
			item.Open = was
		}
	}
}

// sortKeysByRepoDisplayName sorts keys by entity display name from get; falls back to key string.
func sortKeysByRepoDisplayName(keys []string, get func(string) string) {
	sort.Slice(keys, func(i, j int) bool {
		return get(keys[i]) < get(keys[j])
	})
}

func sortPointKeysByDisplayName(p *models.Program, keys []string) {
	repo := ProgramPointRepo(p, config.MainMonitorSizeString)
	sortKeysByRepoDisplayName(keys, func(k string) string {
		if pt, _ := repo.Get(k); pt != nil {
			return pt.Name
		}
		return k
	})
}

func sortSearchAreaKeysByDisplayName(p *models.Program, keys []string) {
	repo := ProgramSearchAreaRepo(p, config.MainMonitorSizeString)
	sortKeysByRepoDisplayName(keys, func(k string) string {
		if sa, _ := repo.Get(k); sa != nil {
			return sa.Name
		}
		return k
	})
}

func sortMaskKeysByDisplayName(p *models.Program, keys []string) {
	repo := ProgramMaskRepo(p)
	sortKeysByRepoDisplayName(keys, func(k string) string {
		if m, _ := repo.Get(k); m != nil {
			return m.Name
		}
		return k
	})
}

// filterKeysByFuzzy returns keys matching filterText (fold match), or the full list when filter is empty.
func filterKeysByFuzzy(filterText string, defaultList []string) []string {
	if filterText == "" {
		return defaultList
	}
	filtered := make([]string, 0, len(defaultList))
	for _, i := range defaultList {
		if fuzzy.MatchFold(filterText, i) {
			filtered = append(filtered, i)
		}
	}
	return filtered
}

// skipProgramAccordionRow mirrors editor accordion visibility: hide a program row when the filter
// matches neither the program name nor any entity name.
func skipProgramAccordionRow(filterText string, programName string, filtered []string) bool {
	return filterText != "" && !fuzzy.MatchFold(filterText, programName) && len(filtered) == 0
}
