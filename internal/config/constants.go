package config

import (
	"log"
	"os"
	"path/filepath"
)

const (
	// User directory structure
	SqyreDir         = "Sqyre"
	UserImagesDir    = "images"
	UserAutoPicDir   = "AutoPic"
	UserIconsDir     = "icons"
	UserMasksDir     = "masks"
	UserMetaDir      = "meta"
	UserVariablesDir = "variables"

	PNG  = ".png"
	JPG  = ".jpg"
	YAML = ".yaml"

	// Icon variant constants
	IconThumbnailSize = 64  // pixels for thumbnail display
	MaxIconVariants   = 100 // maximum variants per item

	//since I have refactored the code to account for multiple programs at once,
	// I need to append the program name to the program properties names,
	// this is the delimiter between the program name and the property name
	// e.g. dark and darker~Health potion (tilde used: Windows disallows "|" in filenames)
	ProgramDelimiter = "~"
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

func GetAutoPicPath() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir, UserImagesDir, UserAutoPicDir)
}

// GetVariablesPath returns the path to the variables directory in the user's home directory
// Returns: ~/Sqyre/variables/
func GetVariablesPath() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir, UserVariablesDir)
}

// GetDbPath returns the path to the config file in the user's home directory.
// Returns: ~/Sqyre/db.yaml
func GetDbPath() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir, "db.yaml")
}

// GetSqyreDir returns the Sqyre application directory in the user's home directory.
// Returns: ~/Sqyre/
func GetSqyreDir() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("Could not get user home directory: %v", err)
	}
	return filepath.Join(homeDir, SqyreDir)
}

// InitializeDirectories creates the necessary directories in the user's home directory
// Creates: ~/Sqyre/images/icons/, ~/Sqyre/variables/, etc.
func InitializeDirectories() error {
	iconsPath := GetIconsPath()
	autoPicPath := GetAutoPicPath()
	variablesPath := GetVariablesPath()

	// Create all parent directories as needed
	if err := os.MkdirAll(iconsPath, 0755); err != nil {
		log.Printf("Failed to create icons directory at %s: %v", iconsPath, err)
		return err
	}

	if err := os.MkdirAll(autoPicPath, 0755); err != nil {
		log.Printf("Failed to create AutoPic directory at %s: %v", autoPicPath, err)
		return err
	}

	if err := os.MkdirAll(variablesPath, 0755); err != nil {
		log.Printf("Failed to create variables directory at %s: %v", variablesPath, err)
		return err
	}

	log.Printf("Initialized directory structure at: %s", iconsPath)
	log.Printf("Initialized AutoPic directory at: %s", autoPicPath)
	return nil
}
