package ui

import (
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"github.com/go-vgo/robotgo"
	hook "github.com/luhrMan/gohook"
)

func (u *Ui) createMainMenu() *fyne.MainMenu {
	// ocrActionMenuItem.Icon, _ = fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")
	macroMenu := fyne.NewMenu("Macro")
	programSelectSubMenu := fyne.NewMenuItem("Select Program", nil)
	actionSubMenu := fyne.NewMenuItem("Add Blank Action", nil)
	basicActionsSubMenu := fyne.NewMenuItem("Basic Actions", nil)
	advancedActionsSubMenu := fyne.NewMenuItem("Advanced Actions", nil)

	macroMenu.Items = append(macroMenu.Items, actionSubMenu, programSelectSubMenu)
	programSelectSubMenu.ChildMenu = fyne.NewMenu("")
	actionSubMenu.ChildMenu = fyne.NewMenu("")

	actionSubMenu.ChildMenu.Items = append(actionSubMenu.ChildMenu.Items,
		basicActionsSubMenu,
		advancedActionsSubMenu,
	)
	addActionAndRefresh :=
		func(a actions.ActionInterface) {
			mt := u.mui.mtabs.selectedTab()
			mt.Macro.Root.AddSubAction(a)
			mt.Select(a.GetUID())
			mt.Refresh()
		}
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Wait", func() { addActionAndRefresh(actions.NewWait(0)) }),
		fyne.NewMenuItem("Mouse Move", func() { addActionAndRefresh(actions.NewMove(coordinates.Point{Name: "", X: 0, Y: 0})) }),
		fyne.NewMenuItem("Click", func() { addActionAndRefresh(actions.NewClick("left")) }),
		fyne.NewMenuItem("Key", func() { addActionAndRefresh(actions.NewKey("ctrl", "down")) }),
	)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("",
		fyne.NewMenuItem("Loop", func() { addActionAndRefresh(actions.NewLoop(1, "", []actions.ActionInterface{})) }),
		fyne.NewMenuItem("Image Search", func() {
			addActionAndRefresh(actions.NewImageSearch(
				"",
				[]actions.ActionInterface{},
				[]string{},
				coordinates.SearchArea{},
			))
		}),
		fyne.NewMenuItem("OCR", func() {
			addActionAndRefresh(actions.NewOcr("", []actions.ActionInterface{}, "", coordinates.SearchArea{}))
		}),
	)

	computerInfo := fyne.NewMenuItem("Computer info", func() {
		var str string
		w, h := robotgo.GetScreenSize()
		str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
		for d := range robotgo.DisplaysNum() {
			_, _, mh, mw := robotgo.GetDisplayBounds(d)
			str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
		}
		dialog.ShowInformation("Computer Information", str, u.win)
	})

	screensize := strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight)
	calibrationMenu := fyne.NewMenu("Coordinate Calibration",
		fyne.NewMenuItem("Everything", func() {
			robotgo.MouseSleep = 0
			robotgo.KeySleep = 0

			coordinates.CalibrateInventorySearchboxes((*programs.GetPrograms())[config.DarkAndDarker].Coordinates[screensize])
			// u.at.boundImageSearchAreaSelect.SetOptions(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice())
			coordinates.CalibrateTopMenuTabLocations((*programs.GetPrograms())[config.DarkAndDarker].Coordinates[screensize])

			mt := u.mui.mtabs.selectedTab()
			robotgo.MouseSleep = mt.Macro.GlobalDelay
			robotgo.KeySleep = mt.Macro.GlobalDelay

		}),
		fyne.NewMenuItem("Top Menu", func() {
			coordinates.CalibrateTopMenuTabLocations((*programs.GetPrograms())[config.DarkAndDarker].Coordinates[screensize])
		}),
		fyne.NewMenuItem("Inventories", func() {
			robotgo.MouseSleep = 0
			robotgo.KeySleep = 0
			coordinates.CalibrateInventorySearchboxes((*programs.GetPrograms())[config.DarkAndDarker].Coordinates[screensize])
			// u.at.boundImageSearchAreaSelect.SetOptions(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice())
			mt := u.mui.mtabs.selectedTab()
			robotgo.MouseSleep = mt.Macro.GlobalDelay
			robotgo.KeySleep = mt.Macro.GlobalDelay
		}),
		fyne.NewMenuItem("Stash-screen", func() {

		}),
		fyne.NewMenuItem("Merchants-screen", func() {

		}),
		fyne.NewMenuItem("Merchants Portraits", func() {
			coordinates.MerchantPortraitsLocation((*programs.GetPrograms())[config.DarkAndDarker].Coordinates[screensize])
		}),
	)

	testMenu := fyne.NewMenu("Test",
		fyne.NewMenuItem("Add Item", func() { addItemWindow() }),
		fyne.NewMenuItem("Set Items from JSON", func() {
			dialog.ShowFileOpen(func(reader fyne.URIReadCloser, err error) {
				log.Println(reader.URI().Path())
				i := items.ParseItemsFromJson(reader.URI().Path())
				for _, item := range i {
					programs.CurrentProgram().Items[item.Name] = item
				}
				items.SetItemsMap(programs.CurrentProgram().Items)
			}, u.win)
		}),
		fyne.NewMenuItem("Test string slice", func() {
			log.Println("String Map:",
				config.ViperConfig.Get("programs"+"."+config.DarkAndDarker+"."+"macros"),
			)
		}),
		fyne.NewMenuItem("unregister failsafe", func() {
			fs := []string{"esc", "ctrl", "shift"}

			hook.Unregister(hook.KeyDown, fs)
		}),
	)

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), macroMenu, calibrationMenu, testMenu)
}
