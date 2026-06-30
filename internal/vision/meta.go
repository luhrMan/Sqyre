package vision

import (
	"Sqyre/internal/config"
	"fmt"
	"log"
	"path/filepath"
	"strings"
	"sync"
	"time"
	"unicode"

	"fyne.io/fyne/v2"
	"gocv.io/x/gocv"
)

var (
	metaImageMu  sync.Mutex
	metaImageSeq uint64
)

func SaveMetaImage(purpose string, img gocv.Mat) {
	saveMetaImage(purpose, img, false)
}

// SaveMetaImageLocked is like SaveMetaImage but must be called while holding the
// OpenCV lock (see WithOpenCV).
func SaveMetaImageLocked(purpose string, img gocv.Mat) {
	saveMetaImage(purpose, img, true)
}

func saveMetaImage(purpose string, img gocv.Mat, openCVLocked bool) {
	app := fyne.CurrentApp()
	if app == nil || !app.Preferences().BoolWithFallback(config.PrefSaveMetaImages, false) {
		return
	}
	if img.Empty() {
		return
	}
	purpose = sanitizeMetaPurpose(purpose)
	if purpose == "" {
		purpose = "image"
	}

	metaImageMu.Lock()
	seq := metaImageSeq
	metaImageSeq++
	ts := time.Now().Format("20060102-150405.000")
	metaImageMu.Unlock()

	filename := fmt.Sprintf("%s-%06d-%s%s", ts, seq, purpose, config.PNG)
	path := filepath.Join(config.GetMetaPath(), filename)
	var ok bool
	write := func() { ok = gocv.IMWrite(path, img) }
	if openCVLocked {
		write()
	} else {
		WithOpenCV(write)
	}
	if !ok {
		log.Printf("meta: failed to write %s", path)
	}
}

func sanitizeMetaPurpose(purpose string) string {
	var b strings.Builder
	for _, r := range purpose {
		if unicode.IsLetter(r) || unicode.IsDigit(r) || r == '-' || r == '_' {
			b.WriteRune(r)
		} else {
			b.WriteRune('-')
		}
	}
	return strings.Trim(b.String(), "-")
}
