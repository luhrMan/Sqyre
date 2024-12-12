package actions

import (
	"Squire/internal/structs"
	"Squire/internal/utils"
	"fmt"
	"image"
	"image/color"
	"log"
	"sort"
	"strings"
	"sync"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

type ImageSearch struct {
	Targets        []string          `json:"imagetargets"`
	SearchBox      structs.SearchBox `json:"searchbox"`
	advancedAction                   //`json:"advancedaction"`
}

func NewImageSearch(name string, subActions []ActionInterface, targets []string, searchbox structs.SearchBox) *ImageSearch {
	return &ImageSearch{
		advancedAction: *newAdvancedAction(name, subActions),
		Targets:        targets,
		SearchBox:      searchbox,
	}
}

func (a *ImageSearch) Execute(ctx interface{}) error {

	log.Printf("Image Search | %v in X1:%d Y1:%d X2:%d Y2:%d", a.Targets, a.SearchBox.LeftX, a.SearchBox.TopY, a.SearchBox.RightX, a.SearchBox.BottomY)
	w := a.SearchBox.RightX - a.SearchBox.LeftX
	h := a.SearchBox.BottomY - a.SearchBox.TopY
	pathDir := "./internal/resources/images/"
	captureImg := robotgo.CaptureImg(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
	img, _ := gocv.ImageToMatRGB(captureImg)
	gocv.IMWrite(pathDir+"search-area.png", img)
	//	img := gocv.IMRead("./images/stash-area.png", gocv.IMReadColor)
	defer img.Close()
	imgDraw := img.Clone()
	defer imgDraw.Close()

	var xSplit, ySplit int
	switch {
	case strings.Contains(a.SearchBox.Name, "Player"):
		xSplit = 5
		ySplit = 10
	case strings.Contains(a.SearchBox.Name, "Stash Inventory"),
		strings.Contains(a.SearchBox.Name, "Merchant Inventory"):
		xSplit = 20
		ySplit = 12
	default:
		xSplit = 1
		ySplit = 1
	}

	var tolerance float32
	switch {
	case strings.Contains(a.SearchBox.Name, "Stash"):
		tolerance = 0.1
	case strings.Contains(a.SearchBox.Name, "Merchant"):
		tolerance = 0.15
	default:
		tolerance = 0.05
	}

	xSize := img.Cols() / ySplit
	ySize := img.Rows() / xSplit
	borderSize := 3
	var splitAreas []image.Rectangle
	for r := 0; r < ySplit; r++ {
		for c := 0; c < xSplit; c++ {
			splitAreas = append(splitAreas, image.Rect((xSize*r)+borderSize, (ySize*c)+borderSize, (xSize+(xSize*r))-borderSize, (ySize+(ySize*c))-borderSize))
		}
	}

	var wg sync.WaitGroup
	results := make(map[string][]robotgo.Point)
	resultsMutex := &sync.Mutex{}
	for _, target := range a.Targets { // for each search target, create a goroutine
		wg.Add(1)
		go func(target string) {
			defer wg.Done()

			ip := pathDir + "icons/" + target + ".png"
			// Read the template
			template := gocv.IMRead(ip, gocv.IMReadColor)
			defer template.Close()
			if template.Empty() {
				fmt.Println("Error reading template image")
				return
			}
			templateCut := template.Region(image.Rect(borderSize, borderSize, template.Cols()-borderSize, template.Rows()-borderSize))

			var colorMatchwg sync.WaitGroup
			var matches []robotgo.Point
			colorMatchResultsMutex := &sync.Mutex{}
			emptyPoint := robotgo.Point{}

			for _, s := range splitAreas { //for each split area, create a goroutine
				colorMatchwg.Add(1)
				go func(s image.Rectangle) {
					defer colorMatchwg.Done()

					var point robotgo.Point
					point = checkHistogramMatch(img.Region(s), templateCut, tolerance, target)
					if point != emptyPoint {
						point = robotgo.Point{X: s.Min.X, Y: s.Min.Y}
						colorMatchResultsMutex.Lock()
						defer colorMatchResultsMutex.Unlock()
						matches = append(matches, point)
					}
				}(s)
			}
			colorMatchwg.Wait()
			//			 matches := findTemplateMatches(img, templateCut, 0.95)
			sort.Slice(matches, func(i, j int) bool {
				return matches[i].Y < matches[j].Y
			})

			resultsMutex.Lock()
			defer resultsMutex.Unlock()
			results[target] = matches
		}(target)
	}
	wg.Wait()
	for i, matches := range results { //draw rectangles around each match
		for _, match := range matches {
			rect := image.Rect(
				match.X,
				match.Y,
				match.X+xSize-(borderSize*2),
				match.Y+ySize-(borderSize*2),
			)
			gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 1)
			gocv.PutText(&imgDraw, i, image.Point{X: match.X, Y: match.Y + ySize}, gocv.FontHersheySimplex, 0.3, color.RGBA{G: 255, A: 255}, 1)
		}
		log.Printf("Results for %s: %v\n", i, matches)
	}
	gocv.IMWrite(pathDir+"founditems.png", imgDraw)
	count := 0
	//clicked := []robotgo.Point
	for _, pointArr := range results {
		for i, point := range pointArr {
			if i > 50 {
				break
			}
			count++
			point.X += a.SearchBox.LeftX
			point.Y += a.SearchBox.TopY
			for _, d := range a.SubActions {
				d.Execute(point)
			}
		}
	}

	log.Printf("Total # found: %v\n", count)
	return nil
}
func (a *ImageSearch) String() string {
	return fmt.Sprintf("%s Image Search for %d items in `%s` | %s", utils.GetEmoji("Image Search"), len(a.Targets), a.SearchBox.Name, a.Name)
}

func checkHistogramMatch(img, template gocv.Mat, tolerance float32, target string) robotgo.Point {
	normType := gocv.NormMinMax
	compType := gocv.HistCmpBhattacharya

	getColorChannels := func(image gocv.Mat, colorModel gocv.ColorConversionCode) gocv.Mat {
		colors := gocv.NewMat()
		gocv.CvtColor(image, &colors, colorModel)
		return colors
	}

	calculateColorModelSimilarities := func(img1, img2 gocv.Mat, bins int) []float32 {
		comparisons := []float32{0, 0, 0}
		img1Channels := gocv.Split(img1)
		img2Channels := gocv.Split(img2)
		for c := range img1Channels {
			gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0, 256}, false)
			gocv.Normalize(img1Channels[c], &img1Channels[c], 0, 1, normType)
			gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0, 256}, false)
			gocv.Normalize(img2Channels[c], &img2Channels[c], 0, 1, normType)
			comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType)
		}
		return comparisons
	}

	calculateHSVColorModelSimilarities := func(img1, img2 gocv.Mat, bins int) []float32 {
		comparisons := []float32{0, 0, 0}
		img1Channels := gocv.Split(img1)
		img2Channels := gocv.Split(img2)
		for c := range img1Channels {
			if c == 0 { //for hue, set range 0 - 180
				gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0, 180}, false)
				gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0, 180}, false)
			} else {
				gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0, 256}, false)
				gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0, 256}, false)
			}
			gocv.Normalize(img1Channels[c], &img1Channels[c], 0, 1, normType)
			gocv.Normalize(img2Channels[c], &img2Channels[c], 0, 1, normType)
			if c == 1 { //for saturation, cut in half idk why it just was going high af
				comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType) / 2
			} else {
				comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType)
			}
		}
		return comparisons
	}

	simBGR := calculateColorModelSimilarities(img, template, 64)

	imgGray := getColorChannels(img, gocv.ColorBGRToGray)
	templateGray := getColorChannels(template, gocv.ColorBGRToGray)
	simGray := calculateColorModelSimilarities(imgGray, templateGray, 64)

	imgHSV := getColorChannels(img, gocv.ColorBGRToHSV)
	templateHSV := getColorChannels(template, gocv.ColorBGRToHSV)
	simHSV := calculateHSVColorModelSimilarities(imgHSV, templateHSV, 64)

	imgLAB := getColorChannels(img, gocv.ColorBGRToLab)
	templateLAB := getColorChannels(template, gocv.ColorBGRToLab)
	simLAB := calculateColorModelSimilarities(imgLAB, templateLAB, 64)

	if
	//	simGray < 0.06 &&
	simBGR[0] < tolerance &&
		simBGR[1] < tolerance &&
		simBGR[2] < tolerance &&
		simHSV[0] < tolerance &&
		simHSV[1] < tolerance &&
		simHSV[2] < tolerance &&
		simLAB[0] < tolerance &&
		simLAB[1] < tolerance &&
		simLAB[2] < tolerance {
		log.Printf("target: %v gray: %.4f\n"+
			"l: %.4f || a: %.4f || b: %.4f\n"+
			"hue: %.4f || sat: %.4f || val: %.4f\n"+
			"blue: %.4f || green: %.4f || red: %.4f",
			target, simGray[0],
			simLAB[0], simLAB[1], simLAB[2],
			simHSV[0], simHSV[1], simHSV[2],
			simBGR[0], simBGR[1], simBGR[2])
		return robotgo.Point{X: img.Size()[0], Y: img.Size()[1]}
	} else {
		return robotgo.Point{}
	}
}

func findTemplateMatches(img, template gocv.Mat, threshold float32) []robotgo.Point {
	// Create the result matrix
	result := gocv.NewMat()
	defer result.Close()

	// Perform template matching
	mask := gocv.NewMat()
	defer mask.Close()

	//	region := template.Region(image.Rect(0, 0, 25, 25))

	//method 5 works best
	gocv.MatchTemplate(img, template, &result, 5, mask)

	//	window := gocv.NewWindow("result")
	//    defer window.Close()
	//	window.IMShow(result)
	//    window.WaitKey(0)
	//
	//	window2 := gocv.NewWindow("region")
	//    defer window2.Close()
	//	window2.IMShow(region)
	//    window2.WaitKey(0)

	// Get the dimensions
	resultRows := result.Rows()
	resultCols := result.Cols()

	var matches []robotgo.Point

	// Iterate through the result matrix and store the matches
	for y := 0; y < resultRows; y++ {
		for x := 0; x < resultCols; x++ {
			confidence := result.GetFloatAt(y, x)
			if confidence >= threshold {
				matches = append(matches, robotgo.Point{
					X: x,
					Y: y,
				})
			}
		}
	}

	return matches
}
