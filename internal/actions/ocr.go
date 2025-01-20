package actions

import (
        "Squire/internal/structs"
        "Squire/internal/utils"
        "bytes"
        "fmt"
        "github.com/go-vgo/robotgo"
        "github.com/otiai10/gosseract/v2"
        "gocv.io/x/gocv"
        "image"
        "image/png"
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
                capture   image.Image
                err       error
                foundText string
        )
        w := a.SearchBox.RightX - a.SearchBox.LeftX
        h := a.SearchBox.BottomY - a.SearchBox.TopY
        if a.SearchBox.Name == "Item Description" {
                capture, err = utils.ItemDescriptionLocation()
                if err != nil {
                        log.Fatal(err)
                }
                err, foundText = CheckImageForText(capture)
                if err != nil {
                        log.Fatal(err)
                }
        } else {
                capture = robotgo.CaptureImg(a.SearchBox.LeftX, a.SearchBox.TopY+h/2, w, h/2)
                err, foundText = CheckImageForText(capture)
                if err != nil {
                        log.Fatal(err)
                }

                if !strings.Contains(foundText, a.Target) {
                        capture = robotgo.CaptureImg(a.SearchBox.LeftX, a.SearchBox.TopY, w, h/2)
                        err, foundText = CheckImageForText(capture)
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

func CheckImageForText(img image.Image) (error, string) {
        client := gosseract.NewClient()
        defer client.Close()
        i, _ := gocv.ImageToMatRGB(img)
        defer i.Close()
        gocv.CvtColor(i, &i, gocv.ColorBGRToGray)
        //        gocv.Threshold(i, &i, 80, 255, gocv.ThresholdBinary)
        gocv.GaussianBlur(i, &i, image.Point{X: 3, Y: 3}, 0, 0, gocv.BorderDefault)
        img, _ = i.ToImage()
        var buf bytes.Buffer
        if err := png.Encode(&buf, img); err != nil {
                return err, ""
        }
        if err := client.SetImageFromBytes(buf.Bytes()); err != nil {
                return err, ""
        }
        text, err := client.Text()
        if err != nil {
                log.Fatal(err)
        }
        return nil, text
}
