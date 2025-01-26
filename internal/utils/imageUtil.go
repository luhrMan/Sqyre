package utils

import (
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/png"
	"log"
	"math"
	"sort"

	"github.com/go-vgo/robotgo"
	"github.com/otiai10/gosseract/v2"
	"gocv.io/x/gocv"
)

var (
	tessClient = gosseract.NewClient()
)

func GetTessClient() *gosseract.Client { return tessClient }
func CloseTessClient()                 { tessClient.Close() }

func GetMatchesFromTemplateMatchResult(result gocv.Mat, threshold float32, closeMatchesDistance int) []robotgo.Point {
	resultRows := result.Rows()
	resultCols := result.Cols()

	var matches []robotgo.Point
	for y := 0; y < resultRows; y++ {
		for x := 0; x < resultCols; x++ {
			confidence := result.GetFloatAt(y, x)
			if math.IsInf(float64(confidence), 0) || math.IsNaN(float64(confidence)) {
				continue
			}
			if confidence >= threshold {
				fmt.Printf("Position (%d, %d), Correlation: %.4f\n",
					x, y, confidence)
				newPoint := robotgo.Point{X: x, Y: y}
				if !isNearExistingPoint(newPoint, matches, closeMatchesDistance) {
					matches = append(matches, newPoint)
				}
			}
		}
	}

	return matches
}

func isNearExistingPoint(point robotgo.Point, matches []robotgo.Point, distance int) bool {
	for _, existing := range matches {
		if abs(existing.X-point.X) <= distance && abs(existing.Y-point.Y) <= distance {
			return true
		}
	}
	return false
}

func abs(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

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

type PreprocessOptions struct {
	BlurAmount   int
	MinThreshold float32
	ResizeScale  float64
}

func ImageToMatToImagePreprocess(img image.Image, gray, blur, threshold, resize bool, ppOptions PreprocessOptions) image.Image {
	i, _ := gocv.ImageToMatRGB(img)
	defer i.Close()
	if ppOptions.BlurAmount == 0 {
		ppOptions.BlurAmount = 3
	}
	if ppOptions.MinThreshold == 0 {
		ppOptions.MinThreshold = 127
	}
	if ppOptions.ResizeScale == 0 {
		ppOptions.ResizeScale = 2
	}
	if gray {
		gocv.CvtColor(i, &i, gocv.ColorBGRToGray)
	}
	if threshold {
		gocv.Threshold(i, &i, ppOptions.MinThreshold, 255, gocv.ThresholdBinary)
	}
	if blur {
		gocv.GaussianBlur(i, &i, image.Point{X: ppOptions.BlurAmount, Y: ppOptions.BlurAmount}, 0, 0, gocv.BorderDefault)
	}
	if resize {
		gocv.Resize(i, &i, image.Point{}, ppOptions.ResizeScale, ppOptions.ResizeScale, gocv.InterpolationDefault)
	}
	gocv.IMWrite("./internal/resources/images/meta/imagetext-test.png", i)
	img, _ = i.ToImage()
	return img
}

func DrawFoundMatches(matches []robotgo.Point, rectSizeX, rectSizeY int, draw gocv.Mat, text string) {
	for _, match := range matches {
		rect := image.Rect(
			match.X,
			match.Y,

			match.X+rectSizeX,
			match.Y+rectSizeY,
		)
		gocv.Rectangle(&draw, rect, color.RGBA{R: 255, A: 255}, 1)
		gocv.PutText(&draw, text, image.Point{X: match.X, Y: match.Y + 5}, gocv.FontHersheySimplex, 0.3, color.RGBA{G: 255, A: 255}, 1)
	}
}

func SortPoints(points []robotgo.Point, sortBy string) []robotgo.Point {
	switch sortBy {
	case "LeftToRightTopToBottom":
	case "RightToLeftTopToBottom":
	case "LeftToRightBottomToTop":
	case "RightToLeftBottomToTop":
	}
	sort.Slice(points, func(i, j int) bool {
		if points[i].Y == points[j].Y {
			return points[i].X < points[j].X
		}
		return points[i].Y < points[j].Y
	})
	return points
}

func SortListOfPoints(lop map[string][]robotgo.Point) []robotgo.Point {
	var sort []robotgo.Point
	for _, points := range lop {
		for _, m := range points {
			sort = append(sort, m)
		}
	}
	return SortPoints(sort, "LeftToRightTopToBottom")
}
