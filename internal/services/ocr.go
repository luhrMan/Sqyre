package services

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
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

var (
	tessClient = gosseract.NewClient()
)

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
	var (
		img       image.Image
		foundText string
	)
	ppOptions := PreprocessOptions{MinThreshold: 50}
	img, err = robotgo.CaptureImg(leftX+config.XOffset, topY+h/2+config.YOffset, w, h/2)
	savedImg, err := gocv.ImageToMatRGB(img)
	if err != nil {
		log.Println("ocr failed:", err)
		return "", err
	}
	gocv.IMWrite(config.GetMetaPath()+"ocr.png", savedImg)

	img = ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
	err, foundText = CheckImageForText(img)
	if err != nil {
		log.Fatal(err)
	}

	if !strings.Contains(foundText, a.Target) {
		img, err = robotgo.CaptureImg(leftX+config.XOffset, topY+config.YOffset, w, h/2)
		img = ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
		err, foundText = CheckImageForText(img)
		if err != nil {
			log.Fatal(err)
		}
	}
	log.Printf("FOUND TEXT: %v", foundText)

	return foundText, nil
}

// func ItemDescriptionLocation() (image.Image, error) {
// 	mx, _ := robotgo.Location()
// 	mx = mx - int(float32(config.MonitorWidth)*0.25)
// 	mw := int(float32(config.MonitorWidth) * 0.50)
// 	if mw+mx > config.MonitorWidth+config.XOffset {
// 		mw = config.MonitorWidth + config.XOffset - mx
// 	}

// 	captureImg := robotgo.CaptureImg(mx, 0, mw, config.MonitorHeight)
// 	img, err := gocv.ImageToMatRGB(captureImg)
// 	if err != nil {
// 		log.Println("Could not convert Image to MatRGB:", err)
// 	}
// 	defer img.Close()
// 	gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+"precorneritemdescription"+config.PNG, img)

// 	trc := gocv.IMRead(config.UpDir+config.UpDir+config.CalibrationImagesPath+"itemCorner-TopRight"+config.PNG, gocv.IMReadColor)
// 	blc := gocv.IMRead(config.UpDir+config.UpDir+config.CalibrationImagesPath+"itemCorner-BottomLeft"+config.PNG, gocv.IMReadColor)
// 	defer trc.Close()
// 	defer blc.Close()
// 	gocv.CvtColor(img, &img, gocv.ColorBGRToGray)
// 	gocv.CvtColor(trc, &trc, gocv.ColorBGRToGray)
// 	gocv.CvtColor(blc, &blc, gocv.ColorBGRToGray)

// 	var threshold float32 = 0.97
// 	result := gocv.NewMat()
// 	defer result.Close()
// 	log.Println("item description")
// 	log.Println("----------------")

// 	trcmatch, err := findCornerCoordinates(img, trc, result, threshold, true)
// 	if err != nil {
// 		return nil, fmt.Errorf("could not find item description | Top Right Corner")
// 	}
// 	log.Println("top right: ", trcmatch)

// 	blcmatch, err := findCornerCoordinates(img, blc, result, threshold, false)
// 	if err != nil {
// 		return nil, fmt.Errorf("could not find item description | Bottom Left Corner")
// 	}
// 	log.Println("bottom left: ", blcmatch)

// 	w := trcmatch[0].X - blcmatch[0].X + 20
// 	h := blcmatch[0].Y - trcmatch[0].Y + 20
// 	x := blcmatch[0].X + mx
// 	y := trcmatch[0].Y + config.YOffset
// 	log.Printf("X: %d, Y: %d, W: %d, H: %d", x, y, w, h)
// 	ci := robotgo.CaptureImg(
// 		x,
// 		y,
// 		w,
// 		h)
// 	i, err := gocv.ImageToMatRGB(ci)
// 	if err != nil {
// 		log.Println("Could not convert Image to MatRGB:", err)
// 	}
// 	defer i.Close()
// 	gocv.IMWrite(config.MetaImagesPath+"itemdescription"+config.PNG, i)

// 	return ci, nil
// }

func findCornerCoordinates(img, corner, result gocv.Mat, threshold float32, resultOffset bool) ([]robotgo.Point, error) {
	gocv.MatchTemplate(img, corner, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
	match := GetMatchesFromTemplateMatchResult(result, threshold, 10)

	switch {
	case len(match) == 1:
		switch resultOffset {
		case true:
			match[0].X = match[0].X + corner.Cols() //resultOffset
			match[0].Y = match[0].Y + corner.Rows() //resultOffset
		case false:
			match[0].X = match[0].X //resultOffset
			match[0].Y = match[0].Y
		}
		return match, nil

	case len(match) > 1:
		return []robotgo.Point{}, fmt.Errorf("found too many matches of corner")
	case len(match) == 0:
		return []robotgo.Point{}, fmt.Errorf("no matches found of corner")
	}

	return nil, fmt.Errorf("unknown error has occured")
}
