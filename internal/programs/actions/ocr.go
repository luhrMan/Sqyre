package actions

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"Squire/internal/utils"
	"fmt"
	"image"
	"log"
	"strings"

	"github.com/go-vgo/robotgo"
)

type Ocr struct {
	Target          string
	SearchArea      coordinates.SearchArea
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewOcr(name string, subActions []ActionInterface, target string, searchbox coordinates.SearchArea) *Ocr {
	return &Ocr{
		AdvancedAction: newAdvancedAction(name, "ocr", subActions),
		Target:         target,
		SearchArea:     searchbox,
	}
}

func (a *Ocr) Execute(ctx any) error {
	log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", config.GetEmoji("OCR"), a.Target, a.SearchArea.LeftX, a.SearchArea.TopY, a.SearchArea.RightX, a.SearchArea.BottomY)
	var (
		img       image.Image
		err       error
		foundText string
	)
	w := a.SearchArea.RightX - a.SearchArea.LeftX
	h := a.SearchArea.BottomY - a.SearchArea.TopY
	ppOptions := utils.PreprocessOptions{MinThreshold: 50}
	if a.SearchArea.Name == "Item Description" {
		img, err = coordinates.ItemDescriptionLocation()
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
			if err != nil {
				log.Fatal(err)
			}
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
	return fmt.Sprintf("%s OCR search for `%s` in `%s`", config.GetEmoji("OCR"), a.Target, a.SearchArea.Name)
}
