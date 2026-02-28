package services

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"bytes"
	"fmt"
	"image"
	"image/png"
	"log"

	"github.com/go-vgo/robotgo"
	"github.com/otiai10/gosseract/v2"
	"gocv.io/x/gocv"
)

var tessClient *gosseract.Client

func init() {
	tessClient = gosseract.NewClient()
	if prefix := assets.EnsureTessdata(); prefix != "" {
		tessClient.SetTessdataPrefix(prefix)
	}
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
		log.Fatal(err)
	}
	return nil, text
}

func OCR(a *actions.Ocr, macro *models.Macro) (string, error) {
	sa := a.SearchArea
	leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
	if err != nil {
		log.Printf("OCR: failed to resolve search area coords: %v", err)
		return "", err
	}
	w := rightX - leftX
	h := bottomY - topY
	if w <= 0 || h <= 0 {
		err := fmt.Errorf("OCR: invalid search area (width=%d height=%d); need positive dimensions", w, h)
		log.Printf("OCR: %v (macro continues)", err)
		return "", err
	}
	log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", a.Target, a.SearchArea.Name, leftX, topY, rightX, bottomY)

	img, err := robotgo.CaptureImg(leftX, topY, w, h)
	if err != nil || img == nil {
		log.Printf("OCR: capture failed: %v", err)
		return "", err
	}
	ppOptions := PreprocessOptions{
		BlurAmount:   a.Blur,
		MinThreshold: float32(a.MinThreshold),
		ResizeScale:  a.Resize,
	}
	img = ImageToMatToImagePreprocess(img, a.Grayscale, a.Blur > 0, a.MinThreshold > 0, a.Resize != 1.0, ppOptions)
	err, foundText := CheckImageForText(img)
	if err != nil {
		log.Fatal(err)
	}
	savedImg, err := gocv.ImageToMatRGB(img)
	if err != nil {
		log.Println("ocr failed:", err)
		return "", err
	}
	gocv.IMWrite(config.GetMetaPath()+"ocr.png", savedImg)

	log.Printf("FOUND TEXT: %v", foundText)

	return foundText, nil
}
