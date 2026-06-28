package editor

import (
	"Sqyre/internal/models/repositories"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

// programListState holds the filtered program keys backing a program list. It is
// stored per tab so list callbacks (Length/UpdateItem/OnSelected) and the search
// handler can be wired exactly once and only the data refreshed afterwards.
type programListState struct {
	filtered []string
}

func setProgramList(list *widget.List) {
	et := shell().EditorTabs.ProgramsTab

	if et.listState != nil {
		// Already wired: just refresh the backing data and the widget.
		applyProgramListFilter(et.listState, currentProgramSearchText(et))
		list.Refresh()
		return
	}

	st := &programListState{filtered: repositories.ProgramRepo().GetAllKeys()}
	et.listState = st

	searchbar := et.Widgets["searchbar"].(*widget.Entry)
	searchbar.SetPlaceHolder("Search here")
	searchbar.OnChanged = func(s string) {
		et.SearchDebouncer().Call(func() {
			applyProgramListFilter(st, searchbar.Text)
			list.ScrollToTop()
			list.Refresh()
		})
	}

	list.Length = func() int {
		return len(st.filtered)
	}
	list.CreateItem = func() fyne.CanvasObject {
		return widget.NewLabel("template")
	}
	list.UpdateItem = func(lii widget.ListItemID, co fyne.CanvasObject) {
		if lii < 0 || lii >= len(st.filtered) {
			return
		}
		co.(*widget.Label).SetText(st.filtered[lii])
	}
	list.OnSelected = func(id widget.ListItemID) {
		if id < 0 || id >= len(st.filtered) {
			return
		}
		et.Widgets["Name"].(*widget.Entry).SetText(st.filtered[id])
		program, err := repositories.ProgramRepo().Get(st.filtered[id])
		if err != nil {
			log.Printf("Error getting program %s: %v", st.filtered[id], err)
			return
		}
		log.Println("selected", program.Name)
		et.SelectedItem = program
		markProgramsClean()
		shell().RefreshEditorActionBar()
	}
}

func currentProgramSearchText(et *EditorTab) string {
	if sb, ok := et.Widgets["searchbar"].(*widget.Entry); ok {
		return sb.Text
	}
	return ""
}

func applyProgramListFilter(st *programListState, search string) {
	defaultList := repositories.ProgramRepo().GetAllKeys()
	if search == "" {
		st.filtered = defaultList
		return
	}
	filtered := make([]string, 0, len(defaultList))
	for _, i := range defaultList {
		if fuzzy.MatchFold(search, i) {
			filtered = append(filtered, i)
		}
	}
	st.filtered = filtered
}
