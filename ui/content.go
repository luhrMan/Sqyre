package ui

import (
	"Squire/internal/data"
	"log"
	"os"
	"strings"

	_ "fyne.io/x/fyne/widget"
	xwidget "fyne.io/x/fyne/widget"

	"fyne.io/fyne/v2/theme"
	widget "fyne.io/fyne/v2/widget"
)

// action settings
var (
	macroName          string
	selectedTreeItem   = ".1"
	time               int
	globalDelay        = 30
	moveX              int
	moveY              int
	spot               string
	button             bool
	key                string
	state              bool
	loopName           string
	count              int = 1
	imageSearchName    string
	searchArea         string
	xSplit             int
	ySplit             int
	imageSearchTargets = data.Items.GetItemsMapAsBool()
	ocrTarget          string
	ocrSearchBox       string
)

func (u *Ui) createSelect() {
	var macroList []string

	getMacroList := func() []string {
		var list []string
		files, err := os.ReadDir(savedMacrosPath)
		if err != nil {
			log.Fatal(err)
		}
		for _, f := range files {
			list = append(list, strings.TrimSuffix(f.Name(), ".json"))
		}
		return list
	}

	macroList = getMacroList()
	u.sel = xwidget.NewCompletionEntry(macroList)
	u.sel.ActionItem = widget.NewButtonWithIcon("", theme.ViewRefreshIcon(), func() { macroList = getMacroList() })
	u.sel.OnSubmitted = func(s string) { u.addMacroDocTab(s) }
	u.sel.OnChanged = func(s string) {
		var matches []string
		userPrefix := strings.ToLower(s)
		for _, listStr := range macroList {
			if len(listStr) < len(s) {
				continue
			}
			listPrefix := strings.ToLower(listStr[:len(s)])
			if userPrefix == listPrefix {
				matches = append(matches, listStr)
			}
		}
		u.sel.SetOptions(matches)
		u.sel.ShowCompletion()
	}
}
