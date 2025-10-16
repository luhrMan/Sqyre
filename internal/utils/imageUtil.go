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

func ImageSearch(sa coordinates.SearchArea, ts []string, rows, cols int) (map[string][]robotgo.Point, error) {
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

	results := match(config.UpDir+config.UpDir+config.MetaImagesPath, img, imgDraw, sa, ts, rows, cols)
	return results, nil

}

func match(pathDir string, img, imgDraw gocv.Mat, sa coordinates.SearchArea, ts []string, rows, cols int) map[string][]robotgo.Point {
	icons := *assets.GetIconBytes()

	//	maskedIcons := *internal.MaskItems()
	results := make(map[string][]robotgo.Point)

	// results = DarkAndDarker(*a, img, imgDraw)

	gocv.IMWrite(pathDir+"founditems.png", imgDraw)

	return results
}

// var (
// icons = *assets.GetIconBytes()
// )

func DarkAndDarker(a ImageSearch, img, imgDraw gocv.Mat) map[string][]robotgo.Point {
	var xSplit, ySplit int
	switch {
	case strings.Contains(a.SearchArea.Name, "player"):
		xSplit = 5
		ySplit = 10
	case strings.Contains(a.SearchArea.Name, config.StashInv),
		strings.Contains(a.SearchArea.Name, config.MerchantInv):
		xSplit = 20
		ySplit = 12
	default:
		xSplit = 1
		ySplit = 1
	}
	xSize := img.Cols() / ySplit
	ySize := img.Rows() / xSplit
	//	var splitAreas []image.Rectangle
	//	for r := 0; r < ySplit; r++ {
	//		for c := 0; c < xSplit; c++ {
	//			splitAreas = append(splitAreas, image.Rect(xSize*r, ySize*c, xSize+(xSize*r), ySize+(ySize*c)))
	//		}
	//	}
	Imask := gocv.NewMat()
	defer Imask.Close()

	var tolerance float32
	switch {
	case strings.Contains(a.SearchArea.Name, config.StashScrPlayerInv):
		tolerance = 0.96
		Imask = gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+config.StashScrPlayerInv+"-"+config.Empty+config.PNG, gocv.IMReadColor)
	case strings.Contains(a.SearchArea.Name, "Stash"):
		tolerance = 0.96
		Imask = gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+config.StashScrStashInv+"-"+config.Empty+config.PNG, gocv.IMReadColor)
	case strings.Contains(a.SearchArea.Name, "Merchant"):
		tolerance = 0.93
		Imask = gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+config.MerchantsScrPlayerInv+"-"+config.Empty+config.PNG, gocv.IMReadColor)
	default:
		tolerance = 0.95
	}

	Tmask1x1 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"1x1 mask"+config.PNG, gocv.IMReadColor)
	Tmask1x2 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"1x2 mask"+config.PNG, gocv.IMReadColor)
	Tmask1x3 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"1x3 mask"+config.PNG, gocv.IMReadColor)
	Tmask2x1 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"2x1 mask"+config.PNG, gocv.IMReadColor)
	Tmask2x2 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"2x2 mask"+config.PNG, gocv.IMReadColor)
	Tmask2x3 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"2x3 mask"+config.PNG, gocv.IMReadColor)
	defer Tmask1x1.Close()
	defer Tmask1x2.Close()
	defer Tmask1x3.Close()
	defer Tmask2x1.Close()
	defer Tmask2x2.Close()
	defer Tmask2x3.Close()
	Cmask1x1 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"1x1 Cmask"+config.PNG, gocv.IMReadGrayScale)
	Cmask1x2 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"1x2 Cmask"+config.PNG, gocv.IMReadGrayScale)
	Cmask1x3 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"1x3 Cmask"+config.PNG, gocv.IMReadGrayScale)
	Cmask2x1 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"2x1 Cmask"+config.PNG, gocv.IMReadGrayScale)
	Cmask2x2 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"2x2 Cmask"+config.PNG, gocv.IMReadGrayScale)
	Cmask2x3 := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"2x3 Cmask"+config.PNG, gocv.IMReadGrayScale)
	defer Cmask1x1.Close()
	defer Cmask1x2.Close()
	defer Cmask1x3.Close()
	defer Cmask2x1.Close()
	defer Cmask2x2.Close()
	defer Cmask2x3.Close()

	results := make(map[string][]robotgo.Point)
	var wg sync.WaitGroup
	resultsMutex := &sync.Mutex{}
	for _, target := range a.Targets { // for each search target, create a goroutine
		wg.Add(1)
		go func(target string) {
			defer wg.Done()

			Tmask := gocv.NewMat()
			Cmask := gocv.NewMat()
			defer Tmask.Close()
			defer Cmask.Close()
			i, err := program.GetProgram.GetItem(target)
			if err != nil {
				log.Println(err)
				return
			}
			switch i.GridSize {
			case [2]int{1, 1}:
				//				Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize))
				Tmask = Tmask1x1.Clone()
				Cmask = Cmask1x1.Clone()
			case [2]int{1, 2}:
				//				Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize*2))
				Tmask = Tmask1x2.Clone()
				Cmask = Cmask1x2.Clone()
			case [2]int{1, 3}:
				//				Tmask = Imask.Region(image.Rect(0, 0, xSize, ySize*3))
				Tmask = Tmask1x3.Clone()
				Cmask = Cmask1x3.Clone()
			case [2]int{2, 1}:
				//				Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize))
				Tmask = Tmask2x1.Clone()
				Cmask = Cmask2x1.Clone()
			case [2]int{2, 2}:
				//				Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize*2))
				Tmask = Tmask2x2.Clone()
				Cmask = Cmask2x2.Clone()
			case [2]int{2, 3}:
				//				Tmask = Imask.Region(image.Rect(0, 0, xSize*2, ySize*3))
				Tmask = Tmask2x3.Clone()
				Cmask = Cmask2x3.Clone()
			}

			ip := target + config.PNG
			b := icons[ip]
			template := gocv.NewMat()
			defer template.Close()
			err = gocv.IMDecodeIntoMat(b, gocv.IMReadColor, &template)
			if err != nil {
				fmt.Println("Error reading template image:", err)
				return
			}

			if Tmask.Cols() != template.Cols() && Tmask.Rows() != template.Rows() {
				log.Println("ERROR: template mask size does not match template!")
				log.Println("item: ", target)
				log.Println("Tmask cols: ", Tmask.Cols())
				log.Println("Tmask rows: ", Tmask.Rows())
				log.Println("t cols: ", template.Cols())
				log.Println("t rows: ", template.Rows())
				return
			}

			var matches []robotgo.Point
			matches = a.FindTemplateMatches(img, template, Imask, Tmask, Cmask, tolerance)

			resultsMutex.Lock()
			defer resultsMutex.Unlock()

			results[target] = matches
			DrawFoundMatches(matches, xSize*i.GridSize[0], ySize*i.GridSize[1], imgDraw, target)
		}(target)
	}
	wg.Wait()

	return results
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
