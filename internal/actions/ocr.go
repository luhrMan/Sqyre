package actions

import (
	"Squire/internal/data"
	"Squire/internal/utils"
	"fmt"
	"image"
	"log"
	"strings"

	"github.com/go-vgo/robotgo"
)

type Ocr struct {
	Target         string          `json:"texttarget"`
	SearchArea     data.SearchArea `json:"searchbox"`
	advancedAction                 //`json:"advancedaction"`
}

func NewOcr(name string, subActions []ActionInterface, target string, searchbox data.SearchArea) *Ocr {
	return &Ocr{
		advancedAction: *newAdvancedAction(name, subActions),
		Target:         target,
		SearchArea:     searchbox,
	}
}

func (a *Ocr) Execute(ctx interface{}) error {
	log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", data.GetEmoji("OCR"), a.Target, a.SearchArea.LeftX, a.SearchArea.TopY, a.SearchArea.RightX, a.SearchArea.BottomY)
	var (
		img       image.Image
		err       error
		foundText string
	)
	w := a.SearchArea.RightX - a.SearchArea.LeftX
	h := a.SearchArea.BottomY - a.SearchArea.TopY
	ppOptions := utils.PreprocessOptions{MinThreshold: 50}
	if a.SearchArea.Name == "Item Description" {
		img, err = data.ItemDescriptionLocation()
		if err != nil {
			log.Fatal(err)
		}
		img = utils.ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
		err, foundText = utils.CheckImageForText(img)
		if err != nil {
			log.Fatal(err)
		}
	} else {
		img = robotgo.CaptureImg(a.SearchArea.LeftX, a.SearchArea.TopY+h/2, w, h/2)
		img = utils.ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
		err, foundText = utils.CheckImageForText(img)
		if err != nil {
			log.Fatal(err)
		}

		if !strings.Contains(foundText, a.Target) {
			img = robotgo.CaptureImg(a.SearchArea.LeftX, a.SearchArea.TopY, w, h/2)
			img = utils.ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
			err, foundText = utils.CheckImageForText(img)
		}
	}

	log.Printf("FOUND TEXT: %v", foundText)
	if strings.Contains(foundText, a.Target) {
		for _, action := range a.SubActions {
			if err := action.Execute(ctx); err != nil {
				return err
			}
		}
	}
	return nil
}

func (a *Ocr) String() string {
	return fmt.Sprintf("%s OCR search for `%s` in `%s`", data.GetEmoji("OCR"), a.Target, a.SearchArea.Name)
}
