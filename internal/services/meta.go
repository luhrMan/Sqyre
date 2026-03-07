package services

import (
	"Squire/internal/config"
	"log"
	"path/filepath"
	"time"

	"fyne.io/fyne/v2"
	"gocv.io/x/gocv"
)

func SaveMetaImage(purpose string, img gocv.Mat) {
	if !fyne.CurrentApp().Preferences().BoolWithFallback(config.PrefSaveMetaImages, false) {
		return
	}
	if img.Empty() {
		return
	}
	ts := time.Now().Format("20060102-150405")
	filename := ts + "-" + purpose + config.PNG
	path := filepath.Join(config.GetMetaPath(), filename)
	if ok := gocv.IMWrite(path, img); !ok {
		log.Printf("meta: failed to write %s", path)
	}
}
