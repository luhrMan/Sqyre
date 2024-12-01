package main

import (
	"Dark-And-Darker/gui"
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"os"

	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
	"github.com/go-vgo/robotgo"
	hook "github.com/robotn/gohook"

	"image"
	"image/color"

	"gocv.io/x/gocv"
)

func main() {
	go func() {
		//	eventHook := hook.Start()
		//var e hook.Event
		//var key string
		ok := hook.AddEvents("f1", "shift", "ctrl")
		if ok {
			log.Println("Exiting...")
			os.Exit(0)
		}
		//		hook.Register(hook.KeyDown, []string{"ctrl", "shift", "f1"}, func(e hook.Event) {
		//			log.Println("Exiting...")
		//			os.Exit(0)
		//		})
		//		s := hook.Start()
		//  		<-hook.Process(s)
		//	    for e = range eventHook {
		//	        if e.Kind == hook.KeyDown {
		//	            key = string(e.Keychar)
		//	            switch key {
		//	            case "s":
		//	                log.Println("pressed k")
		//	            case "l":
		//	                log.Println("pressed l")
		//	            default:
		//	                log.Printf("pressed %s \n", key)
		//	            }
		//	        }
		//	    }
	}()
	a := app.New()
	a.Settings().SetTheme(theme.DarkTheme())
	w := a.NewWindow("Squire")
	icon, _ := fyne.LoadResourceFromPath("./images/Squire.png")
	w.SetIcon(icon)
	w.SetContent(gui.LoadMainContent())
	mainMenu := fyne.NewMainMenu(fyne.NewMenu("Settings"), gui.CreateActionMenu())
	w.SetMainMenu(mainMenu)
	w.ShowAndRun()
}

func stashInventorySlots() {
	img := gocv.IMRead("./images/empty-stash.jpeg", gocv.IMReadColor)
	if img.Empty() {
		fmt.Println("Error reading main image")
	}
	defer img.Close()
	invRows := 20
	invCols := 12
	xSize := img.Cols() / invCols
	ySize := img.Rows() / invRows
	var invSlots []image.Rectangle
	//box := image.Rectangle{image.Point{0,0}, image.Point{img.Rows() / 12, img.Cols() / 24}}
	for r := 0; r < invCols; r++ {
		for c := 0; c < invRows; c++ {
			invSlots = append(invSlots, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
		}
	}
	for _, rect := range invSlots {
		windowSlot := gocv.NewWindow("slot")
		defer windowSlot.Close()
		windowSlot.IMShow(img.Region(rect))
		gocv.WaitKey(0)
		gocv.Rectangle(&img, rect, color.RGBA{R: 255, A: 255}, 2)
	}
	window := gocv.NewWindow("inventory ")
	defer window.Close()
	window.IMShow(img)
	gocv.WaitKey(0)
}

func merchantPlayerInventorySlots() {
	sb := structs.GetSearchBox("Player Inventory Merchant")
	w := sb.RightX - sb.LeftX
	h := sb.BottomY - sb.TopY
	capture := robotgo.CaptureScreen(sb.LeftX+utils.XOffset, sb.TopY+utils.YOffset, w, h)
	robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/search-area.jpeg")
	defer robotgo.FreeBitmap(capture)

	img := gocv.IMRead("./images/search-area.jpeg", gocv.IMReadColor)
	if img.Empty() {
		fmt.Println("Error reading main image")
	}
	defer img.Close()

	invRows := 5
	invCols := 10
	xSize := img.Cols() / invCols
	ySize := img.Rows() / invRows

	//	var invSlotMats []gocv.Mat
	//	for i, s := range invSlots {
	//		invSlotMats = append(invSlotMats, img.Region(s))
	//		defer invSlotMats[i].Close()
	//	}

	var invSlots []image.Rectangle
	for r := 0; r < invCols; r++ {
		for c := 0; c < invRows; c++ {
			invSlots = append(invSlots, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
		}
	}
}
