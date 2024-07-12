// package main

// import (
// 	"Dark-And-Darker/gui"
// 	"log"

// 	"github.com/go-vgo/robotgo"
// 	"github.com/otiai10/gosseract/v2"
// )

// Can't seem to get the resolution of a single display
// 	- Can I just add / subtract the other displays from calculations to ensure proper cursor placement?
// 	- Create a select option in the GUI for this?

// func main() {

// 	log.Println("Screen Size")
// 	log.Println(robotgo.GetScreenSize())
// 	log.Println("Monitor 1 size")
// 	log.Println(robotgo.GetDisplayBounds(0))
// 	log.Println("Monitor 2 size")
// 	log.Println(robotgo.GetDisplayBounds(1))
// 	//gosseractOCR([4]int{0 + XAdditionalMonitorOffset,0 + YAdditionalMonitorOffset, 2560, 300})

// 	gui.Load()
// }

package main

import (
	"Dark-And-Darker/gui"

	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
)

func main() {
	a := app.New()
	a.Settings().SetTheme(theme.DarkTheme())
	w := a.NewWindow("Squire")
	content := gui.LoadMainContent()
	w.SetContent(content)
	w.ShowAndRun()
}
