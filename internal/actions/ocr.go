package actions

import (
        "Squire/internal/structs"
        "Squire/internal/utils"
        "bytes"
        "fmt"
        "github.com/go-vgo/robotgo"
        "github.com/otiai10/gosseract/v2"
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
        client := gosseract.NewClient()
        defer client.Close()

        log.Printf("%s OCR search | %s in X1:%d Y1:%d X2:%d Y2:%d", utils.GetEmoji("OCR"), a.Target, a.SearchBox.LeftX, a.SearchBox.TopY, a.SearchBox.RightX, a.SearchBox.BottomY)
        w := a.SearchBox.RightX - a.SearchBox.LeftX
        h := a.SearchBox.BottomY - a.SearchBox.TopY
        //var text string
        var capture image.Image
        //check bottom first
        capture = robotgo.CaptureImg(a.SearchBox.LeftX, a.SearchBox.TopY+h/2, w, h/2)
        // Convert the capture to an image.Image

        // Encode the image to PNG format in memory
        var buf bytes.Buffer
        if err := png.Encode(&buf, capture); err != nil {
                return err
        }
        if err := client.SetImageFromBytes(buf.Bytes()); err != nil {
                return err
        }

        text, err := client.Text()
        if err != nil {
                log.Fatal(err)
        }
        //if not, check top
        if !strings.Contains(text, a.Target) {
                capture = robotgo.CaptureImg(a.SearchBox.LeftX, a.SearchBox.TopY, w, h/2)

                var buf bytes.Buffer
                if err := png.Encode(&buf, capture); err != nil {
                        return err
                }
                if err := client.SetImageFromBytes(buf.Bytes()); err != nil {
                        return err
                }
                text, err = client.Text()
                if err != nil {
                        log.Fatal(err)
                }
        }
        log.Println("FOUND TEXT:")
        log.Println(text)
        if strings.Contains(text, a.Target) {
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
