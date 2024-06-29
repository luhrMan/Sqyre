package utils

import (
	"Dark-And-Darker/structs"
	"log"

	"github.com/go-vgo/robotgo"
	"github.com/vcaesar/bitmap"
)

// ImageSearch searchBox[x, y, w, h], imagePath "./images/test.png"
func ImageSearch(sbc structs.SearchBoxCoordinates, itemName string) []robotgo.Point {
	//sbc.LeftX += XOffset //might need for linux?
	//sbc.TopY += YOffset

	ip := "./images/" + itemName + ".png"
	capture := robotgo.CaptureScreen(sbc.LeftX, sbc.TopY, sbc.RightX, sbc.BottomY) //sb[0], sb[1], sb[2], sb[3]
	defer robotgo.FreeBitmap(capture)
	robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/wholeScreen.jpeg")

	predefinedImage, err := robotgo.OpenImg(ip)
	if err != nil {
		log.Printf("robotgo.OpenImg failed:%d\n", err)
		return []robotgo.Point{}
	}
	predefinedBitmap := robotgo.ByteToCBitmap(predefinedImage)
	//defer robotgo.FreeBitmap(predefinedBitmap)
	return bitmap.FindAll(predefinedBitmap, capture, 0.2)
}
