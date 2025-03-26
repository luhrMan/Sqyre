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
	DarkAndDarker                = "Dark And Darker"

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
