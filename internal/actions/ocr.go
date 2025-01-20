package actions

import (
        "Squire/internal/structs"
        "Squire/internal/utils"
        "fmt"
        "github.com/go-vgo/robotgo"
        "image"
        "log"
        "strings"
)

type Ocr struct {
        Target         string            `json:"texttarget"`
        SearchBox      structs.SearchBox `json:"searchbox"`
        advancedAction                   //`json:"advancedaction"`
}

func NewOcr(name string, subActions []ActionInterface, target string, searchbox structs.SearchBox) *Ocr {
        return &Ocr{
                advancedAction: *newAdvancedAction(name, subActions),
                Target:         target,
                SearchBox:      searchbox,
        }
}

func (a *Ocr) Execute(ctx interface{}) error {
        log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", utils.GetEmoji("OCR"), a.Target, a.SearchBox.LeftX, a.SearchBox.TopY, a.SearchBox.RightX, a.SearchBox.BottomY)
        var (
                img       image.Image
                err       error
                foundText string
        )
        w := a.SearchBox.RightX - a.SearchBox.LeftX
        h := a.SearchBox.BottomY - a.SearchBox.TopY
        ppOptions := utils.PreprocessOptions{MinThreshold: 50}
        if a.SearchBox.Name == "Item Description" {
                img, err = utils.ItemDescriptionLocation()
                if err != nil {
                        log.Fatal(err)
                }
                img = utils.ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
                err, foundText = utils.CheckImageForText(img)
                if err != nil {
                        log.Fatal(err)
                }
        } else {
                img = robotgo.CaptureImg(a.SearchBox.LeftX, a.SearchBox.TopY+h/2, w, h/2)
                img = utils.ImageToMatToImagePreprocess(img, true, true, false, false, ppOptions)
                err, foundText = utils.CheckImageForText(img)
                if err != nil {
                        log.Fatal(err)
                }

                if !strings.Contains(foundText, a.Target) {
                        img = robotgo.CaptureImg(a.SearchBox.LeftX, a.SearchBox.TopY, w, h/2)
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
        return fmt.Sprintf("%s OCR search for `%s` in `%s`", utils.GetEmoji("OCR"), a.Target, a.SearchBox.Name)
}
