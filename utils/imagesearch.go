package utils

import (
	"Dark-And-Darker/structs"
    "github.com/go-vgo/robotgo"
    "github.com/vcaesar/bitmap"
    "log"
)

// ImageSearch searchBox[x, y, w, h], imagePath "./images/test.png"
func ImageSearch(sbc structs.SearchBoxCoordinates, ip string) (int, int){
	sbc.LeftX += XOffset
	sbc.TopY += YOffset
	
	capture := robotgo.CaptureScreen(sbc.LeftX, sbc.TopY, sbc.RightX, sbc.BottomY) //sb[0], sb[1], sb[2], sb[3]
	defer robotgo.FreeBitmap(capture)
	
	predefinedImage, err := robotgo.OpenImg(ip)
	if err != nil {
    	log.Printf("robotgo.OpenImg failed:%d\n", err)
		return 0, 0
	}
	predefinedBitmap := robotgo.ByteToCBitmap(predefinedImage)

	x, y := bitmap.Find(predefinedBitmap, capture) // add third arg for variance
	//defer robotgo.FreeBitmap(predefinedBitmap)
	if x == -1 && y == -1 {
		log.Println("Predefined image not found in the screenshot.")
	} else {
		log.Printf("Predefined image found at searchBoxCoordinates (x: %d, y: %d)\n", sbc.LeftX + x, sbc.TopY + y)
	}
	return sbc.LeftX + x, sbc.TopY + y
}
