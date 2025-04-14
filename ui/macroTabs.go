package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/macro"
	"errors"
	"log"
	"sort"
	"strconv"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

type macroTabs struct {
	*container.DocTabs
	isExecuting widget.Activity

	boundMacroName      binding.String
	boundMacroNameEntry *widget.Entry

	boundMacroHotkey   binding.ExternalStringList
	macroHotkeySelect1 *widget.Select
	macroHotkeySelect2 *widget.Select
	macroHotkeySelect3 *widget.Select

	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry

	mtMap          map[string]*MacroTree
	boundMacroList binding.StringList
	boundMacroMap  binding.UntypedTree
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

func (mtabs *macroTabs) constructTabs() {
	mtabs.constructMtabSettings()
	mtabs.CreateTab = func() *container.TabItem {
		m := macro.NewMacro("New macro"+strconv.Itoa(len(mtabs.mtMap)), 0, []string{"1", "2", "3"})
		mtabs.SetTreeMapKeyValue(m.Name, &MacroTree{Macro: m, Tree: &widget.Tree{}})
		mtree := mtabs.mtMap[m.Name]
		mtabs.boundMacroList.Append(m.Name)
		mtree.createTree()

		return container.NewTabItem(
			m.Name,
			mtree.Tree,
		)
	}
	mtabs.OnClosed = func(ti *container.TabItem) {
		t := mtabs.mtMap[ti.Text]
		t.UnregisterHotkey()
		delete(mtabs.mtMap, ti.Text)
	}
	mtabs.OnSelected = func(ti *container.TabItem) {
		mt, err := mtabs.selectedTab()
		if err != nil {
			log.Println(err)
			return
		}
		mtabs.boundGlobalDelay.Set(mt.Macro.GlobalDelay)
		mtabs.boundMacroName.Set(mt.Macro.Name)
		mtabs.boundMacroHotkey.Set(mt.Macro.Hotkey)

		mtabs.macroHotkeySelect1.SetSelected(mt.Macro.Hotkey[0])
		mtabs.macroHotkeySelect2.SetSelected(mt.Macro.Hotkey[1])
		mtabs.macroHotkeySelect3.SetSelected(mt.Macro.Hotkey[2])
	}

	for _, m := range programs.GetPrograms().GetProgram(config.DarkAndDarker).Macros {
		mtabs.addTab(m)
	}
}

func (mtabs *macroTabs) selectedTab() (*MacroTree, error) {
	if mtabs == nil || mtabs.Selected() == nil || mtabs.Selected().Text == "" {
		return nil, errors.New("no selected tab")
	}
	macroTree, exists := mtabs.mtMap[mtabs.Selected().Text]
	if !exists {
		return nil, errors.New("selected tab: " + mtabs.Selected().Text + " does not have a corresponding MacroTree")
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
	mtabs.boundMacroList.Append(macro.Name)

	mtree.createTree()

	t := container.NewTabItem(macro.Name, mtree.Tree)
	mtabs.Append(t)
	mtabs.Select(t)

	mtree.RegisterHotkey()

	// boundGlobalDelay.Set(mt.Macro.GlobalDelay)
	// boundMacroName.Set(mt.Macro.Name)
	// boundMacroHotkey.Set(mt.Macro.Hotkey)

	// macroHotkeySelect1.SetSelected(mt.Macro.Hotkey[0])
	// macroHotkeySelect2.SetSelected(mt.Macro.Hotkey[1])
	// macroHotkeySelect3.SetSelected(mt.Macro.Hotkey[2])
	// mtree.Tree.Refresh()
}

func (mtabs *macroTabs) constructMtabSettings() {
	mtabs.boundMacroList = binding.BindStringList(&macroList)
	for _, m := range ui.p.Macros {
		mtabs.boundMacroList.Append(m.Name)
	}
	mtabs.boundMacroList.AddListener(binding.NewDataListener(func() {
		ml, err := mtabs.boundMacroList.Get()
		if err != nil {
			log.Println(err)
			return
		}
		sort.Strings(ml)
	}))

	mtabs.boundMacroName = binding.BindString(&macroName)
	mtabs.boundMacroNameEntry = widget.NewEntryWithData(mtabs.boundMacroName)
	mtabs.boundMacroNameEntry.OnSubmitted = func(string) {
		t, err := mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}
		for _, m := range ui.p.Macros {
			if m.Name == macroName {
				dialog.ShowError(errors.New("macro name already exists"), ui.win)
				return
			}
		}
		delete(mtabs.mtMap, t.Macro.Name)
		mtabs.boundMacroList.Remove(t.Macro.Name)
		mtabs.SetTreeMapKeyValue(macroName, t)
		// u.mtMap[macroName] = t
		t.Macro.Name = macroName
		mtabs.Selected().Text = macroName
		mtabs.boundMacroList.Append(macroName)

		mtabs.Refresh()
	}
	macroHotkey = []string{"1", "2", "3"}
	mtabs.boundMacroHotkey = binding.BindStringList(&macroHotkey)
	mtabs.macroHotkeySelect1 = &widget.Select{Options: []string{"ctrl"}}
	mtabs.macroHotkeySelect2 = &widget.Select{Options: []string{"", "shift"}}
	mtabs.macroHotkeySelect3 = &widget.Select{Options: []string{"1", "2", "3", "4", "5"}}

	mtabs.macroHotkeySelect1.SetSelectedIndex(0)

	mtabs.boundGlobalDelay = binding.BindInt(&globalDelay)
	mtabs.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(mtabs.boundGlobalDelay))
	mtabs.boundGlobalDelay.AddListener(binding.NewDataListener(func() {
		t, err := mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}
		t.Macro.GlobalDelay = globalDelay
		robotgo.MouseSleep = globalDelay
		robotgo.KeySleep = globalDelay
	}))
}
