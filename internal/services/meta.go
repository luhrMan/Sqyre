package services

import (
	"Sqyre/internal/config"
	"log"
	"path/filepath"
	"time"

	"gocv.io/x/gocv"
)

// BoolPreference reads a boolean preference by key with a fallback.
// Set to fyne.CurrentApp().Preferences().BoolWithFallback by the UI layer;
// defaults to always returning the fallback for headless/test use.
var BoolPreference = func(key string, fallback bool) bool { return fallback }

func SaveMetaImage(purpose string, img gocv.Mat) {
	if !BoolPreference(config.PrefSaveMetaImages, false) {
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
