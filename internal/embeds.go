package internal

import (
        "Squire/internal/structs"
        "embed"
        _ "embed"
        "encoding/json"
        "fmt"
        "fyne.io/fyne/v2"
        "log"
)

//go:embed resources/json/items.json
var itemsEmbed []byte

//go:embed resources/images/icons/*
var iconFS embed.FS

var icons = make(map[string][]byte)

var Items structs.Items

func LoadIconBytes() (*map[string][]byte, error) {
        dirPath := "resources/images/icons"
        //        icons := make(map[string][]byte)
        log.Printf("Loading Icon Bytes...")

        entries, err := iconFS.ReadDir(dirPath)
        if err != nil {
                log.Printf("Could not read directory. Error: %v", err)
                return nil, err
        }

        for _, entry := range entries {
                if !entry.IsDir() {
                        iconPath := fmt.Sprintf("%s/%s", dirPath, entry.Name())
                        iconBytes, err := iconFS.ReadFile(iconPath)
                        if err != nil {
                                log.Printf("Could not read image. Error: %v", err)
                                continue
                        }

                        icons[entry.Name()] = iconBytes
                }
        }

        return &icons, nil
}

func GetIconBytes() *map[string][]byte {
        return &icons
}

func BytesToFyneIcons() *map[string]*fyne.StaticResource {
        var iconBytes, _ = LoadIconBytes()
        icons := make(map[string]*fyne.StaticResource)
        for k, v := range *iconBytes {
                icons[k] = fyne.NewStaticResource(k, v)
        }
        return &icons
}
func CreateItemMaps() {
        err := json.Unmarshal(itemsEmbed, &Items.Map)
        if err != nil {
                log.Printf("Error unmarshaling JSON: %v\n", err)
                return
        }
}
