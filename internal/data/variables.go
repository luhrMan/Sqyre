package data

import (
	"github.com/go-vgo/robotgo"
)

const (
	RootPath              string = "./"
	InternalPath                 = RootPath + "internal/"
	DataPath                     = InternalPath + "data/"
	ResourcePath                 = DataPath + "resources/"
	ImagesPath                   = ResourcePath + "images/"
	MetaImagesPath               = ImagesPath + "meta/"
	MaskImagesPath               = ImagesPath + "masks/"
	CalibrationImagesPath        = ImagesPath + "calibration/"
	DarkAndDarker                = "Dark And Darker/"

	Scr                   = "screen"
	Inv                   = "inventory"
	Empty                 = "empty"
	StashScr              = "Stash-" + Scr
	MerchantsScr          = "Merchants-" + Scr
	PlayerInv             = "player-" + Inv
	StashInv              = "stash-" + Inv
	MerchantInv           = "merchant-" + Inv
	StashScrPlayerInv     = StashScr + "-" + PlayerInv
	StashScrStashInv      = StashScr + "-" + StashInv
	MerchantsScrPlayerInv = MerchantsScr + "-" + PlayerInv
	MerchantsScrStashInv  = MerchantsScr + "-" + StashInv

	PNG  = ".png"
	JPG  = ".jpg"
	GOB  = ".gob"
	JSON = ".json"
	YAML = ".yaml"
)

var (
	MainMonitorSize  = robotgo.GetDisplayRect(0)
	MonitorWidth     = MainMonitorSize.W
	MonitorHeight    = MainMonitorSize.H
	XOffset, YOffset = findOffsets()
)

func findOffsets() (X, Y int) {
	for d := range robotgo.DisplaysNum() {
		x, y, _, _ := robotgo.GetDisplayBounds(d)
		if x < 0 {
			X = x * -1
		}
		if y < 0 {
			Y = y * -1
		}
	}
	return X, Y
}
