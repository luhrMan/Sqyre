package config

import (
	"log"
	"os"
	"path/filepath"
)

const (
	RootPath string = "./"
	UpDir           = "../"

	// User directory structure
	SqyreDir           = "Sqyre"
	UserImagesDir      = "images"
	UserIconsDir       = "icons"
	UserMasksDir       = "masks"
	UserMetaDir        = "meta"
	UserCalibrationDir = "calibration"

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

// GetIconsPath returns the path to the icons directory in the user's home directory
// Returns: ~/Sqyre/images/icons/
func GetIconsPath() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir, UserImagesDir, UserIconsDir)
}

func GetMasksPath() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir, UserImagesDir, UserMasksDir)
}

func GetMetaPath() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir, UserImagesDir, UserMetaDir)
}

// InitializeDirectories creates the necessary directories in the user's home directory
// Creates: ~/Sqyre/images/icons/
func InitializeDirectories() error {
	iconsPath := GetIconsPath()

	// Create all parent directories as needed
	if err := os.MkdirAll(iconsPath, 0755); err != nil {
		log.Printf("Failed to create icons directory at %s: %v", iconsPath, err)
		return err
	}

	log.Printf("Initialized directory structure at: %s", iconsPath)
	return nil
}
