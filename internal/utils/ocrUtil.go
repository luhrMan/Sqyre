package utils

import (
	"bytes"
	"image"
	"image/png"
	"log"

	"github.com/otiai10/gosseract/v2"
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
