package main

import (
    "Dark-And-Darker/gui"
    "github.com/go-vgo/robotgo"
    "github.com/otiai10/gosseract/v2"
    "log"
)

// Can't seem to get the resolution of a single display
// 	- Can I just add / subtract the other displays from calculations to ensure proper cursor placement?
// 	- Create a select option in the GUI for this?

func main() {
	log.Println(robotgo.GetScreenSize())
	log.Println(robotgo.GetDisplayBounds(0))
	log.Println(robotgo.GetDisplayBounds(1))
	
//	quitChan := make(chan bool)
//	go func() {
//		for {
//			select {
//			case <-quitChan:
//				return
//			default:
//				robotgo.MilliSleep(1000)
//				log.Println(robotgo.Location())
//			}
//
//		}
//	}()
	//	scrRes := widget.NewLabel("Select your Screen Resolution")
	//	w.SetContent(container.NewVBox(
	//		scrRes,
	//		widget.NewSelect([]string{"2560 x 1440", "1920 x 1080"}, func(value string) {
	//			log.Println("Select set to", value)
	//		}),
	//	))
	//	err := robotgo.ActiveName("Dark and Darker")
	//	if err != nil {
	//	    return
	//	}
	//gosseractOCR([4]int{0 + XAdditionalMonitorOffset,0 + YAdditionalMonitorOffset, 2560, 300})

	//moveMouse := widget.NewLabel("Move mouse")
	gui.Load()
}

func gosseractOCR(sb [4]int) {
	client := gosseract.NewClient()
	defer client.Close()
	//img := robotgo.ToByteImg(robotgo.CaptureImg(sb[0], sb[1], sb[2], sb[3]))
	//capture := robotgo.CaptureImg(sb[0], sb[1], sb[2], sb[3])
	capture := robotgo.CaptureImg([]int{sb[0], sb[1], sb[2], sb[3]}...)
	robotgo.SaveJpeg(capture, "./images/test1.jpeg")
	client.SetImage("./images/test1.jpeg")
	text, _ := client.Text()
	log.Println(text)
	return
}