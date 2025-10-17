package utils

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/program"
	"fmt"
	"image"
	"image/color"
	"log"
	"math"
	"sort"
	"strings"
	"sync"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func ImageSearch(sa coordinates.SearchArea, ts []string, rows, cols int, tolerance float32) (map[string][]robotgo.Point, error) {
	w := sa.RightX - sa.LeftX
	h := sa.BottomY - sa.TopY
	log.Printf("Image Searching | %v in X1:%d Y1:%d X2:%d Y2:%d", ts, sa.LeftX, sa.TopY, sa.RightX, sa.BottomY)

	captureImg := robotgo.CaptureImg(sa.LeftX+config.XOffset, sa.TopY+config.YOffset, w, h)
	img, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		log.Println("image search failed:", err)
		return nil, err
	}
	defer img.Close()
	gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+"search-area.png", img)

	imgDraw := img.Clone()
	defer imgDraw.Close()

	results := match(config.UpDir+config.UpDir+config.MetaImagesPath, img, imgDraw, sa, ts, rows, cols, tolerance)
	return results, nil

}

func match(pathDir string, img, imgDraw gocv.Mat, sa coordinates.SearchArea, ts []string, rows, cols int, tolerance float32) map[string][]robotgo.Point {
	icons := *assets.GetIconBytes()
	xSize := img.Cols() / cols
	ySize := img.Rows() / rows

	Imask := gocv.NewMat()
	defer Imask.Close()

	results := make(map[string][]robotgo.Point)
	var wg sync.WaitGroup
	resultsMutex := &sync.Mutex{}
	for _, t := range ts { // for each search target, create a goroutine
		wg.Add(1)
		go func(t string) {
			defer wg.Done()
			p := program.GetProgram(strings.Split(t, config.ProgramDelimiter)[0])
			i, err := p.Items.GetItem(strings.Split(t, config.ProgramDelimiter)[1])
			if err != nil {
				log.Println(err)
				return
			}

			//LOAD IN MASKS HERE

			// Tmask := gocv.NewMat()
			// Cmask := gocv.NewMat()
			// defer Tmask.Close()
			// defer Cmask.Close()

			// switch i.GridSize {
			// case [2]int{1, 1}:
			// 	//				Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize))
			// 	Tmask = Tmask1x1.Clone()
			// 	Cmask = Cmask1x1.Clone()
			// case [2]int{1, 2}:
			// 	//				Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize*2))
			// 	Tmask = Tmask1x2.Clone()
			// 	Cmask = Cmask1x2.Clone()
			// case [2]int{1, 3}:
			// 	//				Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize*3))
			// 	Tmask = Tmask1x3.Clone()
			// 	Cmask = Cmask1x3.Clone()
			// case [2]int{2, 1}:
			// 	//				Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize))
			// 	Tmask = Tmask2x1.Clone()
			// 	Cmask = Cmask2x1.Clone()
			// case [2]int{2, 2}:
			// 	//				Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize*2))
			// 	Tmask = Tmask2x2.Clone()
			// 	Cmask = Cmask2x2.Clone()
			// case [2]int{2, 3}:
			// 	//				Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize*3))
			// 	Tmask = Tmask2x3.Clone()
			// 	Cmask = Cmask2x3.Clone()
			// }

			ip := t + config.PNG
			b := icons[ip]
			template := gocv.NewMat()
			defer template.Close()
			err = gocv.IMDecodeIntoMat(b, gocv.IMReadColor, &template)
			if err != nil {
				fmt.Println("Error reading template image:", err)
				return
			}

			// if Tmask.Cols() != template.Cols() && Tmask.Rows() != template.Rows() {
			// 	log.Println("ERROR: template mask size does not match template!")
			// 	log.Println("item: ", t)
			// 	log.Println("Tmask cols: ", Tmask.Cols())
			// 	log.Println("Tmask rows: ", Tmask.Rows())
			// 	log.Println("t cols: ", template.Cols())
			// 	log.Println("t rows: ", template.Rows())
			// 	return
			// }

			var matches []robotgo.Point
			matches = FindTemplateMatches(img, template, Imask, gocv.NewMat(), gocv.NewMat(), tolerance) //Tmask, Cmask, tolerance)

			resultsMutex.Lock()
			defer resultsMutex.Unlock()

			results[t] = matches
			DrawFoundMatches(matches, xSize*i.GridSize[0], ySize*i.GridSize[1], imgDraw, t)
		}(t)
	}
	wg.Wait()
	//	maskedIcons := *internal.MaskItems()

	gocv.IMWrite(pathDir+"founditems.png", imgDraw)

	return results
}

func FindTemplateMatches(img, template, Imask, Tmask, Cmask gocv.Mat, threshold float32) []robotgo.Point {
	result := gocv.NewMat()
	defer result.Close()

	i := img.Clone()
	t := template.Clone()
	defer i.Close()
	defer t.Close()
	kernel := image.Point{X: 5, Y: 5}

	// if Imask.Rows() > 0 && Imask.Cols() > 0 {
	// 	gocv.Subtract(i, Imask, &i)
	// 	gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+"imageSubtraction.png", i)
	// }
	// if Tmask.Rows() > 0 && Tmask.Cols() > 0 {
	// 	gocv.Subtract(t, Tmask, &t)
	// 	gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+"templateSubtraction.png", t)
	// }

	gocv.GaussianBlur(i, &i, kernel, 0, 0, gocv.BorderDefault)
	gocv.GaussianBlur(t, &t, kernel, 0, 0, gocv.BorderDefault)

	//method 5 works best
	gocv.MatchTemplate(i, t, &result, gocv.TemplateMatchMode(5), Cmask)
	matches := GetMatchesFromTemplateMatchResult(result, threshold, 10)
	return matches
}

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

type PreprocessOptions struct {
	BlurAmount   int
	MinThreshold float32
	ResizeScale  float64
}

func ImageToMatToImagePreprocess(img image.Image, gray, blur, threshold, resize bool, ppOptions PreprocessOptions) image.Image {
	i, err := gocv.ImageToMatRGB(img)
	if err != nil {
		log.Println(err)
		return nil
	}
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
		gocv.Threshold(i, &i, ppOptions.MinThreshold, 255, gocv.ThresholdBinaryInv)
	}
	if blur {
		gocv.GaussianBlur(i, &i, image.Point{X: ppOptions.BlurAmount, Y: ppOptions.BlurAmount}, 0, 0, gocv.BorderDefault)
	}
	if resize {
		gocv.Resize(i, &i, image.Point{}, ppOptions.ResizeScale, ppOptions.ResizeScale, gocv.InterpolationDefault)
	}
	gocv.IMWrite("./internal/resources/images/meta/imagetext-test.png", i)
	img, err = i.ToImage()
	if err != nil {
		log.Println(err)
		return nil
	}
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
	case "TopLeftToBottomRight":
		sort.Slice(points, func(i, j int) bool {
			for a := 0; a <= 5; a++ {
				if points[i].Y+a == points[j].Y || points[i].Y == points[j].Y+a {
					return points[i].X < points[j].X
				}
			}
			return points[i].Y < points[j].Y
		})
	case "TopRightToBottomLeft":
	case "BottomLeftToTopRight":
	case "BottomRightToTopLeft":
	}

	return points
}

func SortListOfPoints(lop map[string][]robotgo.Point) []robotgo.Point {
	var sort []robotgo.Point
	for _, points := range lop {
		sort = append(sort, points...)
	}
	return SortPoints(sort, "TopLeftToBottomRight")
}
