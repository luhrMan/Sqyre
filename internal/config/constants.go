package config

const (
	RootPath              string = "./"
	UpDir                        = "../"
	InternalPath                 = "internal/"
	AssetsPath                   = InternalPath + "assets/"
	ImagesPath                   = AssetsPath + "images/"
	IconsPath                    = ImagesPath + "icons/"
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

	// Icon variant constants
	IconThumbnailSize = 64  // pixels for thumbnail display
	MaxIconVariants   = 100 // maximum variants per item

	//since I have refactored the code to account for multiple programs at once,
	// I need to append the program name to the program properties names,
	// this is the delimiter between the program name and the property name
	// e.g. dark and darker|Health potion
	ProgramDelimiter = "|"
)
