package vision

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"bytes"
	"fmt"
	"image"
	"image/png"
	"log"
	"strings"
	"sync"
	"time"

	"fyne.io/fyne/v2"
	"github.com/otiai10/gosseract/v2"
	"gocv.io/x/gocv"
)

var (
	tessClient *gosseract.Client
	tessOnce   sync.Once
	tessWarmOnce sync.Once
)

func GetTessClient() *gosseract.Client {
	tessOnce.Do(func() {
		tessClient = gosseract.NewClient()
		_ = tessClient.SetTessdataFromMemory(assets.EngTrainedData())
		_ = tessClient.SetLanguage("eng")
	})
	return tessClient
}

// WarmUpOCR loads embedded tessdata and initializes Tesseract so the first OCR
// action in a macro does not pay one-time startup cost.
func WarmUpOCR() {
	tessWarmOnce.Do(func() {
		start := time.Now()
		if err := GetTessClient().WarmUp(); err != nil {
			log.Printf("OCR warmup: %v", err)
			return
		}
		log.Printf("OCR engine ready in %s", time.Since(start).Round(time.Millisecond))
	})
}

func MacroUsesOCR(m *models.Macro) bool {
	seen := make(map[string]bool)
	return macroUsesOCRRec(m, seen)
}

func macroUsesOCRRec(m *models.Macro, seen map[string]bool) bool {
	if m == nil || m.Root == nil {
		return false
	}
	if m.Name != "" {
		if seen[m.Name] {
			return false
		}
		seen[m.Name] = true
	}
	var uses bool
	models.WalkActions(m.Root, func(a actions.ActionInterface) {
		if uses {
			return
		}
		if _, ok := a.(*actions.Ocr); ok {
			uses = true
			return
		}
		rm, ok := a.(*actions.RunMacro)
		if !ok || rm.MacroName == "" {
			return
		}
		target, err := repositories.MacroRepo().Get(rm.MacroName)
		if err == nil && macroUsesOCRRec(target, seen) {
			uses = true
		}
	})
	return uses
}

func CloseTessClient() {
	if tessClient != nil {
		tessClient.Close()
	}
}

func releaseTessClientImage() {
	client := GetTessClient()
	client.ClearAdaptiveClassifier()
	client.Clear()
	client.ClearPixImage()
}

func setTessImageFromMat(mat gocv.Mat) error {
	if mat.Empty() {
		return fmt.Errorf("empty OCR image")
	}
	var err error
	WithOpenCV(func() {
		err = setTessImageFromMatLocked(mat)
	})
	return err
}

func setTessImageFromMatLocked(mat gocv.Mat) error {
	working := mat
	if mat.Channels() == 3 {
		rgb := gocv.NewMat()
		defer rgb.Close()
		gocv.CvtColor(mat, &rgb, gocv.ColorBGRToRGB)
		working = rgb
	}
	data, err := working.DataPtrUint8()
	if err != nil {
		return err
	}
	ch := working.Channels()
	if ch <= 0 {
		return fmt.Errorf("invalid OCR image channels")
	}
	return GetTessClient().SetRawImage(data, working.Cols(), working.Rows(), ch, working.Step())
}

func setTessImageFromGo(img image.Image) error {
	var err error
	WithOpenCV(func() {
		mat, matErr := gocv.ImageToMatRGB(img)
		if matErr != nil {
			err = matErr
			return
		}
		defer mat.Close()
		err = setTessImageFromMatLocked(mat)
	})
	return err
}

func CheckImageForText(img image.Image) (error, string) {
	defer releaseTessClientImage()
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		return err, ""
	}
	if err := GetTessClient().SetImageFromBytes(buf.Bytes()); err != nil {
		return err, ""
	}
	text, err := GetTessClient().Text()
	if err != nil {
		return err, ""
	}
	return nil, text
}

func ocrMatWithBoxes(mat gocv.Mat) (text string, boxes []gosseract.BoundingBox, err error) {
	if err := setTessImageFromMat(mat); err != nil {
		return "", nil, err
	}
	boxes, err = GetTessClient().GetBoundingBoxes(gosseract.RIL_WORD)
	if err != nil {
		return "", nil, err
	}
	text = textFromOCRBoxes(boxes)
	return text, boxes, nil
}

// ocrImageWithBoxes runs OCR on a preprocessed Go image.
func ocrImageWithBoxes(img image.Image) (text string, boxes []gosseract.BoundingBox, err error) {
	mat, err := gocv.ImageToMatRGB(img)
	if err != nil {
		return "", nil, err
	}
	defer mat.Close()
	return ocrMatWithBoxes(mat)
}

func textFromOCRBoxes(boxes []gosseract.BoundingBox) string {
	var b strings.Builder
	for _, box := range boxes {
		word := strings.TrimSpace(box.Word)
		if word == "" {
			continue
		}
		if b.Len() > 0 {
			b.WriteByte(' ')
		}
		b.WriteString(word)
	}
	return strings.Trim(b.String(), "\n")
}

// findTargetInBoxes returns the center (in image coords) of the bounding box(es) that match the target text.
// If multiple words match (e.g. target "Submit Button"), returns the center of the union of their boxes.
// ok is false if no matching word box was found.
func findTargetInBoxes(boxes []gosseract.BoundingBox, target string) (centerX, centerY int, ok bool) {
	target = strings.TrimSpace(target)
	if target == "" {
		return 0, 0, false
	}
	targetLower := strings.ToLower(target)
	targetWords := strings.Fields(targetLower)
	var matching []image.Rectangle
	for _, b := range boxes {
		word := strings.TrimSpace(b.Word)
		if word == "" {
			continue
		}
		wordLower := strings.ToLower(word)
		matched := strings.Contains(targetLower, wordLower) || strings.Contains(wordLower, targetLower)
		if !matched && len(targetWords) > 0 {
			for _, tw := range targetWords {
				if strings.Contains(wordLower, tw) || strings.Contains(tw, wordLower) {
					matched = true
					break
				}
			}
		}
		if matched {
			matching = append(matching, b.Box)
		}
	}
	if len(matching) == 0 {
		return 0, 0, false
	}
	// Union of all matching boxes, then center
	minX, minY := matching[0].Min.X, matching[0].Min.Y
	maxX, maxY := matching[0].Max.X, matching[0].Max.Y
	for _, r := range matching[1:] {
		if r.Min.X < minX {
			minX = r.Min.X
		}
		if r.Min.Y < minY {
			minY = r.Min.Y
		}
		if r.Max.X > maxX {
			maxX = r.Max.X
		}
		if r.Max.Y > maxY {
			maxY = r.Max.Y
		}
	}
	cx := (minX + maxX) / 2
	cy := (minY + maxY) / 2
	return cx, cy, true
}

// OCR runs OCR on the action's search area and returns the found text plus the screen X,Y of where the
// target text was found (center of its bounding box). If the target is not found in word boxes,
// returns the center of the search area as fallback.
func OCR(a *actions.Ocr, macro *models.Macro) (foundText string, outX, outY int, err error) {
	defer func() {
		if r := recover(); r != nil {
			macropkg.LogPanicToFile(r, "OCR")
			foundText, outX, outY = "", 0, 0
			err = fmt.Errorf("OCR panic: %v", r)
			log.Printf("OCR: %v (macro continues)", err)
		}
	}()
	return ocrCapture(a, macro)
}

func ocrCapture(a *actions.Ocr, macro *models.Macro) (foundText string, outX, outY int, err error) {
	defer releaseTessClientImage()
	resolvedLeftX, resolvedTopY, resolvedRightX, resolvedBottomY, err := macropkg.ResolveSearchAreaCoordsFromRef(a.SearchArea, macro, macropkg.DefaultResolutionKey())
	if err != nil {
		log.Printf("OCR: failed to resolve search area %q: %v", a.SearchArea, err)
		return "", 0, 0, err
	}
	img, leftX, topY, rightX, bottomY, err := macropkg.CaptureSearchArea(resolvedLeftX, resolvedTopY, resolvedRightX, resolvedBottomY)
	if err != nil {
		log.Printf("OCR: %v (macro continues)", err)
		return "", 0, 0, err
	}
	searchCenterX := (leftX + rightX) / 2
	searchCenterY := (topY + bottomY) / 2
	log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.SearchArea.DisplayLabel(), leftX, topY, rightX, bottomY)
	ppOptions := PreprocessOptions{
		Grayscale:       a.Grayscale,
		Blur:            a.Blur > 0,
		BlurAmount:      a.Blur,
		Threshold:       a.MinThreshold > 0 || a.ThresholdOtsu,
		MinThreshold:    float32(a.MinThreshold),
		ThresholdOtsu:   a.ThresholdOtsu,
		ThresholdInvert: a.ThresholdInvert,
		Resize:          a.Resize != 1.0,
		ResizeScale:     a.Resize,
	}
	mat, err := preprocessCaptureMat(img, ppOptions)
	if err != nil {
		log.Printf("OCR: image preprocessing failed: %v (macro continues)", err)
		return "", 0, 0, fmt.Errorf("OCR: image preprocessing failed")
	}
	defer mat.Close()
	foundText, boxes, checkErr := ocrMatWithBoxes(mat)
	if checkErr != nil {
		return "", 0, 0, checkErr
	}
	if fyne.CurrentApp().Preferences().BoolWithFallback(config.PrefSaveMetaImages, false) {
		SaveMetaImage("ocr", mat)
	}

	log.Printf("FOUND TEXT: %d chars", len(foundText))

	// Resolve position of the target text on screen
	outX = searchCenterX
	outY = searchCenterY
	resizeScale := a.Resize
	if resizeScale <= 0 {
		resizeScale = 1.0
	}
	if boxCenterX, boxCenterY, ok := findTargetInBoxes(boxes, a.Target); ok {
		// Bounding box coords are in preprocessed image space; convert to screen
		outX = leftX + int(float64(boxCenterX)/resizeScale)
		outY = topY + int(float64(boxCenterY)/resizeScale)
		log.Printf("OCR target %q position: screen (%d, %d)", a.Target, outX, outY)
	}

	return foundText, outX, outY, nil
}
