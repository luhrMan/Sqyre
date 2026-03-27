package editor

import (
	"Sqyre/internal/models/repositories"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

func setProgramList(list *widget.List) {
	et := shell().EditorTabs.ProgramsTab
	searchbar := et.Widgets["searchbar"].(*widget.Entry)
	var filtered = repositories.ProgramRepo().GetAllKeys()

	searchbar.SetPlaceHolder("Search here")
	searchbar.OnChanged = func(s string) {
		defaultList := repositories.ProgramRepo().GetAllKeys()
		defer list.ScrollToTop()
		defer list.Refresh()

		if s == "" {
			filtered = defaultList
			return
		}
		filtered = []string{}
		for _, i := range defaultList {
			if fuzzy.MatchFold(s, i) {
				filtered = append(filtered, i)
			}
		}
	}

	list.Length = func() int {
		return len(filtered)
	}
	list.CreateItem = func() fyne.CanvasObject {
		return widget.NewLabel("template")
	}
	list.UpdateItem = func(lii widget.ListItemID, co fyne.CanvasObject) {
		label := co.(*widget.Label)
		pname := filtered[lii]
		label.SetText(pname)
	}
	list.OnSelected = func(id widget.ListItemID) {
		et.Widgets["Name"].(*widget.Entry).SetText(filtered[id])
		program, err := repositories.ProgramRepo().Get(filtered[id])
		if err != nil {
			log.Printf("Error getting program %s: %v", filtered[id], err)
			return
		}
		log.Println("selected", program.Name)
		et.SelectedItem = program
		markProgramsClean()
		shell().RefreshEditorActionBar()
	}
}
