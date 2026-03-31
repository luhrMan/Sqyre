//go:build !js

package services

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"bytes"
	"fmt"
	"image"
	"image/png"
	"log"
	"strings"

	"github.com/go-vgo/robotgo"
	"github.com/otiai10/gosseract/v2"
	"gocv.io/x/gocv"
)

var tessClient *gosseract.Client

func init() {
	tessClient = gosseract.NewClient()
	// Initialize from embedded traineddata in memory only (no filesystem writes).
	_ = tessClient.SetTessdataFromMemory(assets.EngTrainedData())
	_ = tessClient.SetLanguage("eng")
}

func GetTessClient() *gosseract.Client { return tessClient }
func CloseTessClient()                 { tessClient.Close() }

func CheckImageForText(img image.Image) (error, string) {
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

// ocrImage runs OCR on the preprocessed image and returns full text plus word-level bounding boxes.
// The client must have the image set; boxes are in preprocessed image pixel coordinates.
func ocrImageWithBoxes(img image.Image) (text string, boxes []gosseract.BoundingBox, err error) {
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		return "", nil, err
	}
	if err := GetTessClient().SetImageFromBytes(buf.Bytes()); err != nil {
		return "", nil, err
	}
	text, err = GetTessClient().Text()
	if err != nil {
		return "", nil, err
	}
	boxes, err = GetTessClient().GetBoundingBoxes(gosseract.RIL_WORD)
	if err != nil {
		return text, nil, err
	}
	return text, boxes, nil
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
	sa := a.SearchArea
	leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
	if err != nil {
		log.Printf("OCR: failed to resolve search area coords: %v", err)
		return "", 0, 0, err
	}
	w := rightX - leftX
	h := bottomY - topY
	if w <= 0 || h <= 0 {
		err := fmt.Errorf("OCR: invalid search area (width=%d height=%d); need positive dimensions", w, h)
		log.Printf("OCR: %v (macro continues)", err)
		return "", 0, 0, err
	}
	searchCenterX := (leftX + rightX) / 2
	searchCenterY := (topY + bottomY) / 2
	log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.SearchArea.Name, leftX, topY, rightX, bottomY)

	img, err := robotgo.CaptureImg(leftX, topY, w, h)
	if err != nil || img == nil {
		log.Printf("OCR: capture failed: %v", err)
		return "", 0, 0, err
	}
	ppOptions := PreprocessOptions{
		BlurAmount:   a.Blur,
		MinThreshold: float32(a.MinThreshold),
		ResizeScale:  a.Resize,
	}
	img = ImageToMatToImagePreprocess(img, a.Grayscale, a.Blur > 0, a.MinThreshold > 0, a.Resize != 1.0, ppOptions)
	foundText, boxes, checkErr := ocrImageWithBoxes(img)
	if checkErr != nil {
		return "", 0, 0, checkErr
	}
	savedImg, err := gocv.ImageToMatRGB(img)
	if err != nil {
		log.Println("ocr failed:", err)
		return "", 0, 0, err
	}
	SaveMetaImage("ocr", savedImg)

	log.Printf("FOUND TEXT: %v", foundText)

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
