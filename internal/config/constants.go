package config

import (
	"Sqyre/internal/logger"
	"os"
	"path/filepath"
)

// sqyreDirFallback is used when user home cannot be determined (no panic).
var sqyreDirFallback string

func getSqyreDir() string {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		if sqyreDirFallback == "" {
			sqyreDirFallback = filepath.Join(os.TempDir(), "sqyre")
			logger.Errorf("Could not get user home directory: %v; using fallback: %s", err, sqyreDirFallback)
		}
		return sqyreDirFallback
	}
	return filepath.Join(homeDir, SqyreDir)
}

const (
	// User directory structure
	SqyreDir         = ".sqyre"
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

	// Fyne preference keys
	PrefSaveMetaImages  = "save_meta_images"
	PrefEnabledMonitors = "enabled_monitors" // comma-separated indices, e.g. "0,2"; empty = all (must match screen package key)
	PrefWindowX         = "window_x"
	PrefWindowY         = "window_y"
	PrefWindowWidth     = "window_width"
	PrefWindowHeight    = "window_height"

	//since I have refactored the code to account for multiple programs at once,
	// I need to append the program name to the program properties names,
	// this is the delimiter between the program name and the property name
	// e.g. dark and darker~Health potion (tilde used: Windows disallows "|" in filenames)
	ProgramDelimiter     = "~"
	DescriptionDelimiter = " / "
)

// GetIconsPath returns the path to the icons directory in the user's home directory.
// Returns ~/.sqyre/images/icons/ or a fallback under os.TempDir() if home is unavailable.
func GetIconsPath() string {
	return filepath.Join(getSqyreDir(), UserImagesDir, UserIconsDir)
}

func GetMasksPath() string {
	return filepath.Join(getSqyreDir(), UserImagesDir, UserMasksDir)
}

func GetMetaPath() string {
	return filepath.Join(getSqyreDir(), UserImagesDir, UserMetaDir)
}

func GetAutoPicPath() string {
	return filepath.Join(getSqyreDir(), UserImagesDir, UserAutoPicDir)
}

// GetVariablesPath returns the path to the variables directory.
// Returns ~/.sqyre/variables/ or fallback if home is unavailable.
func GetVariablesPath() string {
	return filepath.Join(getSqyreDir(), UserVariablesDir)
}

// GetDbPath returns the path to the config file. Returns ~/.sqyre/db.yaml or fallback.
func GetDbPath() string {
	return filepath.Join(getSqyreDir(), "db.yaml")
}

// GetSqyreDir returns the Sqyre application directory.
// Returns ~/.sqyre/ or a fallback under os.TempDir() if user home cannot be determined (no panic).
func GetSqyreDir() string {
	return getSqyreDir()
}

// InitializeDirectories creates the necessary directories in the user's home directory
// Creates: ~/.sqyre/images/icons/, ~/.sqyre/variables/, etc.
func InitializeDirectories() error {
	iconsPath := GetIconsPath()
	autoPicPath := GetAutoPicPath()
	variablesPath := GetVariablesPath()

	metaPath := GetMetaPath()

	// Create all parent directories as needed
	if err := os.MkdirAll(iconsPath, 0755); err != nil {
		logger.Errorf("Failed to create icons directory at %s: %v", iconsPath, err)
		return err
	}

	if err := os.MkdirAll(autoPicPath, 0755); err != nil {
		logger.Errorf("Failed to create AutoPic directory at %s: %v", autoPicPath, err)
		return err
	}

	if err := os.MkdirAll(variablesPath, 0755); err != nil {
		logger.Errorf("Failed to create variables directory at %s: %v", variablesPath, err)
		return err
	}

	if err := os.MkdirAll(metaPath, 0755); err != nil {
		logger.Errorf("Failed to create meta directory at %s: %v", metaPath, err)
		return err
	}

	logger.Infof("Initialized directory structure at: %s", iconsPath)
	logger.Infof("Initialized AutoPic directory at: %s", autoPicPath)
	logger.Infof("Initialized meta directory at: %s", metaPath)
	return nil
}
