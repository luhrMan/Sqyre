package utils

import (
	"Dark-And-Darker/structs"
	"log"

	"github.com/go-vgo/robotgo"
	"github.com/vcaesar/bitmap"
)

// ImageSearch searchBox[x, y, w, h], imagePath "./images/test.png"
func ImageSearch(sbc structs.SearchBoxCoordinates, itemName string) (int, int) {
	sbc.LeftX += XOffset
	sbc.TopY += YOffset
	ip := "./images/" + itemName + ".png"
	capture := robotgo.CaptureScreen(sbc.LeftX, sbc.TopY, sbc.RightX, sbc.BottomY) //sb[0], sb[1], sb[2], sb[3]
	defer robotgo.FreeBitmap(capture)

	predefinedImage, err := robotgo.OpenImg(ip)
	if err != nil {
		log.Printf("robotgo.OpenImg failed:%d\n", err)
		return 0, 0
	}
	predefinedBitmap := robotgo.ByteToCBitmap(predefinedImage)
	defer robotgo.FreeBitmap(predefinedBitmap)

	x, y := bitmap.Find(predefinedBitmap, capture, 0.1) // add third arg for variance
	if x == -1 && y == -1 {
		log.Printf("%s image not found in the screenshot.", itemName)
	} else {
		log.Printf("%s found at searchBoxCoordinates (x: %d, y: %d)\n", itemName, sbc.LeftX+x, sbc.TopY+y)
	}
	return x, y
}
