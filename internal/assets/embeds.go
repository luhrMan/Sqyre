package assets

import (
	"embed"

	"fmt"
	"log"

	"fyne.io/fyne/v2"
)

//go:embed images/Squire.png
var appIcon []byte
var AppIcon = fyne.NewStaticResource("appIcon", appIcon)

//go:embed images/icons/*
var iconFS embed.FS

var icons = make(map[string][]byte)

func LoadIconBytes() (*map[string][]byte, error) {
	dirPath := "images/icons"
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
	i := make(map[string]*fyne.StaticResource)
	for s, b := range *iconBytes {
		i[s] = fyne.NewStaticResource(s, b)
	}
	return &i
}

//func MaskItems() *map[string][]byte {
//        //        icons = *GetIconBytes()
//        maskedIcons := make(map[string][]byte)
//        Imask := gocv.IMRead("./internal/resources/images/empty-stash.png", gocv.IMReadColor)
//        Tmask := gocv.NewMat()
//        defer Imask.Close()
//        defer Tmask.Close()
//        xSplit := 20
//        ySplit := 12
//        xSize := Imask.Cols() / ySplit
//        ySize := Imask.Rows() / xSplit
//
//        for s, b := range icons {
//                s = strings.TrimSuffix(s, filepath.Ext(s))
//                //                ip := s + ".png"
//                //                b := icons[ip]
//                t := gocv.NewMat()
//                defer t.Close()
//                err := gocv.IMDecodeIntoMat(b, gocv.IMReadColor, &t)
//                if err != nil {
//                        fmt.Println("Error reading template image:", s)
//                        fmt.Println(err)
//                        continue
//                }
//                i, _ := Items.GetItem(s)
//                if i == nil {
//                        log.Println("failed to load: ", s)
//                        continue
//                }
//                switch i.GridSize {
//                case [2]int{1, 1}:
//                        Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize))
//                case [2]int{1, 2}:
//                        Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize*2))
//                case [2]int{1, 3}:
//                        Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize*3))
//                case [2]int{2, 1}:
//                        Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize))
//                case [2]int{2, 2}:
//                        Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize*2))
//                case [2]int{2, 3}:
//                        Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize*3))
//                }
//                if Tmask.Cols() != t.Cols() && Tmask.Rows() != t.Rows() {
//                        log.Println("item:", s)
//                        log.Println("Tmask cols:", Tmask.Cols())
//                        log.Println("Tmask rows:", Tmask.Rows())
//                        log.Println("t cols:", t.Cols())
//                        log.Println("t rows:", t.Rows())
//                        continue
//                }
//                gocv.Subtract(t, Tmask, &t)
//                maskedIcons[s] = t.ToBytes()
//        }
//        return &maskedIcons
//}
