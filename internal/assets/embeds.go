package assets

import (
	"Squire/internal/config"
	"embed"

	"fmt"
	"log"

	"fyne.io/fyne/v2"
)

//go:embed images/icon.svg
var appIcon []byte
var AppIcon = fyne.NewStaticResource("appIcon", appIcon)

//go:embed images/icons/*
var iconFS embed.FS

var icons = make(map[string][]byte)

func LoadIconBytes() (map[string][]byte, error) {
	dirPath := "images/icons"
	//        icons := make(map[string][]byte)
	log.Printf("Loading Icon Bytes...")

	entries, err := iconFS.ReadDir(dirPath)
	if err != nil {
		log.Printf("Could not read directory. Error: %v", err)
		return nil, err
	}

	for _, entry := range entries {
		if entry.IsDir() {
			subentries, err := iconFS.ReadDir(dirPath + "/" + entry.Name())
			if err != nil {
				log.Printf("Could not read directory. Error: %v", err)
				return nil, err
			}
			for _, se := range subentries {
				iconPath := fmt.Sprintf("%s/%s", dirPath, entry.Name()+"/"+se.Name())
				iconBytes, err := iconFS.ReadFile(iconPath)
				if err != nil {
					log.Printf("Could not read image. Error: %v", err)
					continue
				}
				icons[entry.Name()+config.ProgramDelimiter+se.Name()] = iconBytes
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
