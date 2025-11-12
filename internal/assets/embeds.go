package assets

import (
	"Squire/internal/config"
	_ "embed"
	"log"
	"os"
	"path/filepath"

	"fyne.io/fyne/v2"
)

//go:embed images/icon.svg
var appIcon []byte
var AppIcon = fyne.NewStaticResource("appIcon", appIcon)

var icons = make(map[string][]byte)

func LoadIconBytes() (map[string][]byte, error) {
	log.Printf("Loading Icon Bytes...")

	iconsPath := config.GetIconsPath()

	// Read program directories from filesystem
	entries, err := os.ReadDir(iconsPath)
	if err != nil {
		// Graceful degradation if directory doesn't exist
		if os.IsNotExist(err) {
			log.Printf("Icons directory does not exist: %s", iconsPath)
			return icons, nil
		}
		log.Printf("Could not read directory %s. Error: %v", iconsPath, err)
		return nil, err
	}

	for _, entry := range entries {
		if entry.IsDir() {
			programName := entry.Name()
			programPath := filepath.Join(iconsPath, programName)

			subentries, err := os.ReadDir(programPath)
			if err != nil {
				log.Printf("Could not read directory %s. Error: %v", programPath, err)
				continue
			}

			for _, se := range subentries {
				if se.IsDir() {
					continue
				}

				iconPath := filepath.Join(programPath, se.Name())
				iconBytes, err := os.ReadFile(iconPath)
				if err != nil {
					log.Printf("Could not read icon %s. Error: %v", iconPath, err)
					continue
				}

				// Store icon with program delimiter and filename
				// This handles both variant files (ItemName|VariantName.png)
				// and non-variant files (ItemName.png) correctly
				icons[programName+config.ProgramDelimiter+se.Name()] = iconBytes
			}
		}
	}

	return icons, nil
}

// func CustomArrowUpIcon() []byte {
// 	moveup, err := fyne.LoadResourceFromPath("MoveUp.svg")
// 	if err != nil {
// 		log.Println(err)
// 	}
// 	return moveup.Content()
// }

func GetIconBytes() map[string][]byte {
	return icons
}

func BytesToFyneIcons() map[string]*fyne.StaticResource {
	var iconBytes, _ = LoadIconBytes()
	i := make(map[string]*fyne.StaticResource)
	for s, b := range iconBytes {
		i[s] = fyne.NewStaticResource(s, b)
	}
	return i
}
