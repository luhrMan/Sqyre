package config

const (
	RootPath              string = "./"
	UpDir                        = "../"
	InternalPath                 = "internal/"
	AssetsPath                   = InternalPath + "assets/"
	ImagesPath                   = AssetsPath + "images/"
	MetaImagesPath               = ImagesPath + "meta/"
	MaskImagesPath               = ImagesPath + "masks/"
	CalibrationImagesPath        = ImagesPath + "calibration/"
	DarkAndDarker                = "dark and darker"

	Scr                   = "screen"
	Inv                   = "inventory"
	Empty                 = "empty"
	StashScr              = "stash-" + Scr
	MerchantsScr          = "merchants-" + Scr
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

var Emojis = map[string]string{
	"Move":         "↔️",
	"Click":        "🖱️",
	"Key":          "⌨️",
	"Wait":         "⏳",
	"Image Search": "🔍",
	"OCR":          "🔬",
	"Loop":         "🔁",
	"Conditional":  "❓",
}

func GetEmoji(key string) string {
	return Emojis[key]
}
