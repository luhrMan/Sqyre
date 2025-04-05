package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/macro"
	"Squire/internal/utils"
	"errors"
	"log"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
	hook "github.com/robotn/gohook"
)

type macroTabs struct {
	*container.DocTabs
	isExecuting widget.Activity

	mtMap          map[string]*MacroTree
	boundMacroList binding.StringList
}

func (mtabs *macroTabs) SetTreeMapKeyValue(key string, mt *MacroTree) { mtabs.mtMap[key] = mt }

func (mtabs *macroTabs) GetTabTree() (*MacroTree, error) {
	mtree, err := mtabs.selectedTab()
	if err != nil {
		return nil, err
	}
	if mtree == nil {
		return nil, errors.New("macroTree is nil")
	}
	if mtree.Tree == nil {
		return nil, errors.New("macroTree Tree is nil")
	}
	if mtree.Macro == nil {
		return nil, errors.New("macroTree Macro is nil")
	}
	if mtree.Macro.Root == nil {
		return nil, errors.New("macroTree Macro Root is nil")
	}
	return mtree, nil
}

func (mui *macroUi) constructTabs() {
	mui.mtabs.OnClosed = func(ti *container.TabItem) {
		delete(mui.mtabs.mtMap, ti.Text)
		mui.mtabs.ReRegisterHotkeys()
	}
	mui.mtabs.OnSelected = func(ti *container.TabItem) {
		mt, err := mui.mtabs.selectedTab()
		if err != nil {
			log.Println(err)
			return
		}
		mui.boundGlobalDelay.Set(mt.Macro.GlobalDelay)
		mui.boundMacroName.Set(mt.Macro.Name)
		mui.boundMacroHotkey.Set(mt.Macro.Hotkey)

		mui.macroHotkeySelect1.SetSelected(mt.Macro.Hotkey[0])
		mui.macroHotkeySelect2.SetSelected(mt.Macro.Hotkey[1])
		mui.macroHotkeySelect3.SetSelected(mt.Macro.Hotkey[2])
	}

	mui.mtabs.Items = append(mui.mtabs.Items, container.NewTabItem("", container.NewBorder(nil, nil, nil, nil)))
	mui.mtabs.SelectIndex(0)

	for _, m := range programs.GetPrograms().GetProgram(config.DarkAndDarker).Macros {
		mui.mtabs.addTab(m)
	}

	mui.mtabs.RemoveIndex(0)
	mui.mtabs.SelectIndex(0)

}

func (mtabs *macroTabs) selectedTab() (*MacroTree, error) {
	if mtabs == nil || mtabs.Selected() == nil {
		return nil, errors.New("no selected tab")
	}
	macroTree, exists := mtabs.mtMap[mtabs.Selected().Text]
	if !exists {
		return nil, errors.New("selected tab does not have a corresponding MacroTree")
	}

	return macroTree, nil
}

func (mtabs *macroTabs) addTab(macro *macro.Macro) {
	//check if already open. if it is, select it.
	if _, ok := mtabs.mtMap[macro.Name]; ok {
		log.Println("macro is already open")
		for _, d := range mtabs.Items {
			if d.Text == macro.Name {
				mtabs.Select(d)
			}
		}
		return
	}

	mtabs.SetTreeMapKeyValue(macro.Name, &MacroTree{Macro: macro, Tree: &widget.Tree{}})
	mtree := mtabs.mtMap[macro.Name]

	mtree.createTree()

	t := container.NewTabItem(macro.Name, mtree.Tree)
	mtabs.Append(t)
	mtabs.Select(t)

	mtree.setUpdateTreeOnselect()

	log.Println("this is happening when adding tab:", macro.Name)
	mtabs.ReRegisterHotkeys()
	mtree.Tree.Refresh()
}

func (mtabs *macroTabs) RegisterHotkeys() {
	for _, m := range mtabs.mtMap {
		hk := make([]string, len(m.Macro.Hotkey))
		copy(hk, m.Macro.Hotkey)
		if hk[1] == "" {
			hk = append(hk[:1], hk[1+1:]...)
		}

		hook.Register(hook.KeyDown, hk, func(e hook.Event) {
			log.Println("pressed", hk)
			m.Macro.ExecuteActionTree()
		})
		log.Println("registered:", m.Macro.Hotkey)
	}
}

func (mtabs *macroTabs) ReRegisterHotkeys() {
	hook.End()
	log.Println("hook ended")
	utils.FailsafeHotkey()
	mtabs.RegisterHotkeys()
	go utils.StartHook()
}
