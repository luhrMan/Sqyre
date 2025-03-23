package ui

import (
	"Squire/internal"
	"Squire/internal/data"
	"log"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func (u *Ui) createDocTabs() {
	u.dt = container.NewDocTabs()
	u.dt.OnClosed = func(ti *container.TabItem) { delete(u.mtm, ti.Text) }
	for _, m := range internal.GetPrograms().GetProgram(data.DarkAndDarker).Macros {
		u.addMacroDocTab(m)
	}
	u.dt.SelectIndex(0)
}

func (u *Ui) selectedMacroTab() *MacroTree {
	if len(u.dt.Items) == 0 {
		log.Println("No tabs, selecting first macro")
		u.addMacroDocTab(internal.GetPrograms().GetProgram(data.DarkAndDarker).Macros[0])
		u.dt.SelectIndex(0)
	}
	return u.mtm[u.dt.Selected().Text]
}

func (u *Ui) addMacroDocTab(macro *internal.Macro) {
	u.AddMacroTree(macro.Name, &MacroTree{Macro: macro, Tree: &widget.Tree{}})
	if _, ok := u.mtm[macro.Name]; !ok {
		return
	}
	mt := u.mtm[macro.Name]

	mt.createTree()

	t := container.NewTabItem(macro.Name, mt.Tree)
	u.dt.Append(t)
	u.dt.Select(t)
	u.updateTreeOnselect()
	mt.Tree.Refresh()
}
