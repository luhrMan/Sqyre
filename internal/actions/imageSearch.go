package actions

import (
	"Squire/internal"
	"Squire/internal/structs"
	"Squire/internal/utils"
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

type ImageSearch struct {
	Targets        []string          `json:"imagetargets"`
	SearchBox      structs.SearchBox `json:"searchbox"`
	advancedAction                   //`json:"advancedaction"`
}

type Match struct {
	Location image.Point
	Score    float64
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
	captureImg := robotgo.CaptureImg(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()
	pathDir := "internal/resources/images/"
	gocv.IMWrite(pathDir+"search-area.png", img)

	imgDraw := img.Clone()
	defer imgDraw.Close()

	results := a.match("template", pathDir, img, imgDraw)

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

func (a *ImageSearch) match(matchMode, pathDir string, img, imgDraw gocv.Mat) map[string][]robotgo.Point {
	icons := *internal.GetIconBytes()

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
	Imask := gocv.NewMat()
	var tolerance float32
	switch {
	case strings.Contains(a.SearchBox.Name, "Player Inventory Stash"):
		tolerance = 0.96
		Imask = gocv.IMRead("./internal/resources/images/empty-player-stash.png", gocv.IMReadColor)
	case strings.Contains(a.SearchBox.Name, "Stash"):
		tolerance = 0.96
		Imask = gocv.IMRead("./internal/resources/images/empty-stash.png", gocv.IMReadColor)
	case strings.Contains(a.SearchBox.Name, "Merchant"):
		tolerance = 0.93
		Imask = gocv.IMRead("./internal/resources/images/empty-player-merchant.png", gocv.IMReadColor)
	default:
		tolerance = 0.95
	}

	xSize := img.Cols() / ySplit
	ySize := img.Rows() / xSplit
	borderSize := 0
	var splitAreas []image.Rectangle
	for r := 0; r < ySplit; r++ {
		for c := 0; c < xSplit; c++ {
			splitAreas = append(splitAreas, image.Rect((xSize*r)+borderSize, (ySize*c)+borderSize, (xSize+(xSize*r))-borderSize, (ySize+(ySize*c))-borderSize))
		}
	}
	Tmask := gocv.IMRead("./internal/resources/images/1x1 mask.png", gocv.IMReadColor)
	defer Imask.Close()
	defer Tmask.Close()
	results := make(map[string][]robotgo.Point)
	var wg sync.WaitGroup
	resultsMutex := &sync.Mutex{}
	for _, target := range a.Targets { // for each search target, create a goroutine
		wg.Add(1)
		go func(target string) {
			defer wg.Done()

			ip := target + ".png"
			b := icons[ip]
			template := gocv.NewMat()
			defer template.Close()
			err := gocv.IMDecodeIntoMat(b, gocv.IMReadColor, &template)
			if err != nil {
				fmt.Println("Error reading template image")
				fmt.Println(err)
				return
			}

			templateCut := template.Region(image.Rect(borderSize, borderSize, template.Cols()-borderSize, template.Rows()-borderSize))

			var matches []robotgo.Point
			switch matchMode {
			case "template":
				matches = a.findTemplateMatches(img, template, Imask, Tmask, tolerance)
			case "color":
				matches = a.colorMatching(img, templateCut, tolerance, target, splitAreas)
			case "threshold":
				a.thresholdMatching(img, template)
			case "feature":
				matches = a.featureMatching(img, template, target)
			default:

			}
			sort.Slice(matches, func(i, j int) bool {
				return matches[i].Y < matches[j].Y
			})

			resultsMutex.Lock()
			defer resultsMutex.Unlock()

			results[target] = matches

			for _, match := range matches {
				rect := image.Rect(
					match.X,
					match.Y,

					match.X+xSize-(borderSize*2),
					match.Y+ySize-(borderSize*2),
				)
				gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 1)
				gocv.PutText(&imgDraw, target, image.Point{X: match.X, Y: match.Y + 5}, gocv.FontHersheySimplex, 0.3, color.RGBA{G: 255, A: 255}, 1)
			}
		}(target)
	}
	wg.Wait()

	//	for i, matches := range results { //draw rectangles around each match
	//		for _, match := range matches {
	//			rect := image.Rect(
	//				match.X,
	//				match.Y,
	//
	//				match.X+25-(borderSize*2),
	//				match.Y+25-(borderSize*2),
	//			)
	//			gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 1)
	//			gocv.PutText(&imgDraw, i, image.Point{X: match.X, Y: match.Y + 25}, gocv.FontHersheySimplex, 0.3, color.RGBA{G: 255, A: 255}, 1)
	//		}
	//		log.Printf("Results for %s: %v\n", i, matches)
	//	}
	gocv.IMWrite(pathDir+"founditems.png", imgDraw)

	return results
}

func (a *ImageSearch) colorMatching(img, templateCut gocv.Mat, tolerance float32, target string, splitAreas []image.Rectangle) []robotgo.Point {
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
	return matches
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

func (a *ImageSearch) findTemplateMatches(img, template, Imask, Tmask gocv.Mat, threshold float32) []robotgo.Point {
	result := gocv.NewMat()
	defer result.Close()
	mask := gocv.IMRead("./internal/resources/images/icons/mask1.png", gocv.IMReadGrayScale)
	defer mask.Close()

	i := img.Clone()
	t := template.Clone()
	defer i.Close()
	defer t.Close()
	kernel := image.Point{X: 5, Y: 5}

	gocv.Subtract(i, Imask, &i)
	gocv.Subtract(t, Tmask, &t)
	gocv.GaussianBlur(i, &i, kernel, 0, 0, gocv.BorderDefault)
	gocv.GaussianBlur(t, &t, kernel, 0, 0, gocv.BorderDefault)

	//method 5 works best
	gocv.MatchTemplate(i, t, &result, gocv.TemplateMatchMode(5), mask)

	resultRows := result.Rows()
	resultCols := result.Cols()

	var matches []robotgo.Point

	// Iterate through the result matrix and store the matches
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
				if !isNearExistingPoint(newPoint, matches, 5) {
					matches = append(matches, newPoint)
				}
			}
		}
	}

	return matches
}
func isNearExistingPoint(point robotgo.Point, matches []robotgo.Point, distance int) bool {
	for _, existing := range matches {
		// Check if the point is within the distance threshold
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

func (a *ImageSearch) featureMatching(img, template gocv.Mat, target string) []robotgo.Point {
	//	sift := gocv.NewSIFT()
	nFeatures := 0
	nOctaveLayers := 5
	contrastThreshold := 0.04
	edgeThreshold := 300.0
	sigma := 1.6
	sift := gocv.NewSIFTWithParams(&nFeatures, &nOctaveLayers, &contrastThreshold, &edgeThreshold, &sigma)
	defer sift.Close()

	mask := gocv.NewMat()
	defer mask.Close()

	m := gocv.IMRead("./internal/resources/images/empty-stash.png", gocv.IMReadGrayScale)
	defer m.Close()
	diffed := gocv.NewMat()
	defer diffed.Close()
	threshM := gocv.NewMat()
	defer threshM.Close()

	grayI := gocv.NewMat()
	threshI := gocv.NewMat()
	bitwiseI := gocv.NewMat()
	defer grayI.Close()
	defer threshI.Close()
	defer bitwiseI.Close()
	gocv.CvtColor(img, &grayI, gocv.ColorBGRToGray)
	gocv.AbsDiff(m, grayI, &diffed)
	gocv.Threshold(diffed, &threshM, 48, 255, gocv.ThresholdBinary)
	kernel := gocv.GetStructuringElement(gocv.MorphCross, image.Point{1, 1})
	defer kernel.Close()
	gocv.MorphologyExWithParams(diffed, &diffed, gocv.MorphType(gocv.MorphCross), kernel, 1, gocv.BorderIsolated)
	gocv.Inpaint(img, diffed, &diffed, 1, 1)

	//	gocv.Threshold(grayI, &threshI, 48, 255, gocv.ThresholdBinary)
	//	gocv.BitwiseAndWithMask(img, img, &bitwiseI, threshI)

	grayT := gocv.NewMat()
	threshT := gocv.NewMat()
	bitwiseT := gocv.NewMat()
	defer grayT.Close()
	defer threshT.Close()
	defer bitwiseT.Close()
	gocv.CvtColor(template, &grayT, gocv.ColorBGRToGray)
	gocv.Threshold(grayT, &threshT, 48, 255, gocv.ThresholdBinary)
	gocv.BitwiseAndWithMask(template, template, &bitwiseT, threshT)

	//	kp1, des1 := sift.DetectAndCompute(bitwiseI, mask)
	kp1, des1 := sift.DetectAndCompute(diffed, mask)
	gocv.DrawKeyPoints(diffed, kp1, &img, color.RGBA{R: 255}, gocv.NotDrawSinglePoints)
	w := gocv.NewWindow("test")
	defer w.Close()
	w.IMShow(img)
	w.WaitKey(0)

	kp2, des2 := sift.DetectAndCompute(bitwiseT, mask)
	matcher := gocv.NewBFMatcher()
	//	matcher := gocv.NewFlannBasedMatcher()
	defer matcher.Close()

	matches := matcher.KnnMatch(des1, des2, 2)

	//	var tolerance float64
	//	switch {
	//	case strings.Contains(a.SearchBox.Name, "Stash"):
	//		tolerance = 0.15
	//	case strings.Contains(a.SearchBox.Name, "Merchant"):
	//		tolerance = 0.2
	//	default:
	//		tolerance = 0.05
	//	}

	var goodMatches []gocv.DMatch
	for _, m := range matches {
		if len(m) > 1 {
			if m[0].Distance < 0.1*m[1].Distance {
				goodMatches = append(goodMatches, m[0])
			}
		}
	}

	//	locationMap := make(map[image.Point]int)

	// Apply ratio test and group matches by location
	//	for _, m := range matches {
	//		if len(m) >= 2 && m[0].Distance < 0.2*m[1].Distance {
	//			// Get the location of the match in the search image
	//			matchLoc := kp1[m[0].TrainIdx]
	//
	//			// Round to nearest coordinate to group nearby matches
	//			roundedPoint := image.Point{
	//				X: int(matchLoc.X),
	//				Y: int(matchLoc.Y),
	//			}
	//
	//			// Group matches within a small radius
	//			found := false
	//			for existingPoint := range locationMap {
	//				dx := float64(existingPoint.X - roundedPoint.X)
	//				dy := float64(existingPoint.Y - roundedPoint.Y)
	//				distance := dx*dx + dy*dy
	//
	//				if distance < 25 { // Adjust this radius based on your icon size
	//					locationMap[existingPoint]++
	//					found = true
	//					break
	//				}
	//			}
	//
	//			if !found {
	//				locationMap[roundedPoint] = 1
	//			}
	//		}
	//	}

	// Convert to matches array, filtering by minimum match count
	//	var results []Match
	var points []robotgo.Point

	//	for loc, count := range locationMap {
	//		if count >= minMatchCount {
	//			results = append(results, Match{
	//				Location: loc,
	//				Score:    float64(count) / float64(len(matches)),
	//			})
	//			points = append(points, robotgo.Point{
	//				X: loc.X,
	//				Y: loc.Y,
	//			})
	//		}
	//	}
	log.Println(target)
	log.Println(points)
	draw := img.Clone()
	if len(goodMatches) > 1 {
		gocv.DrawMatches(bitwiseI, kp1, bitwiseT, kp2, goodMatches, &draw, color.RGBA{255, 0, 0, 0}, color.RGBA{0, 255, 0, 0}, nil, gocv.NotDrawSinglePoints)
	}
	//	w := fyne.CurrentApp().NewWindow("found images")
	gocv.IMWrite("./internal/resources/images/FM/"+target+"FM.png", draw)

	//	w.Content(canvas.NewImageFromFile())

	return points
}

func (a *ImageSearch) thresholdMatching(img, template gocv.Mat) {
	log.Println("Threshold matching...")
	//	grayImg := gocv.NewMat()
	//	defer grayImg.Close()

	grayTemplate := gocv.NewMat()
	defer grayTemplate.Close()

	thTemplate := gocv.NewMat()
	defer thTemplate.Close()

	//	gocv.CvtColor(img, &grayImg, gocv.ColorRGBToGray)
	gocv.CvtColor(template, &grayTemplate, gocv.ColorRGBToGray)

	gocv.AdaptiveThreshold(grayTemplate, &thTemplate, 50, gocv.AdaptiveThresholdGaussian, gocv.ThresholdBinary, 11, 2)
	window := gocv.NewWindow("Threshold")
	defer window.Close()
	window.IMShow(thTemplate)
	gocv.WaitKey(0)
}

//func (a *ImageSearch) bitMapMatching(captureImg image.Image, pathDir string) map[string][]robotgo.Point {
//	robotgo.ToCBitmap(robotgo.ImgToBitmap(captureImg))
//	icons := *internal.GetIconBytes()
//	var wg sync.WaitGroup
//	results := make(map[string][]robotgo.Point)
//	resultsMutex := &sync.Mutex{}
//	for _, target := range a.Targets { // for each search target, create a goroutine
//		wg.Add(1)
//		go func(target string) {
//			defer wg.Done()
//
//			templateBytes := icons[target]
//			templateImg := robotgo.ByteToCBitmap(templateBytes)
//			defer robotgo.FreeBitmap(templateImg)
//
//			var (
//				matches []robotgo.Point
//				point   robotgo.Point
//			)
//
//			point =
//
//				matches = append(matches, point)
//
//			sort.Slice(matches, func(i, j int) bool {
//				return matches[i].Y < matches[j].Y
//			})
//
//			resultsMutex.Lock()
//			defer resultsMutex.Unlock()
//			results[target] = matches
//		}(target)
//	}
//
//	for i, matches := range results { //draw rectangles around each match
//		for _, match := range matches {
//			rect := image.Rect(
//				match.X,
//				match.Y,
//				match.X+xSize-(borderSize*2),
//				match.Y+ySize-(borderSize*2),
//			)
//			gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 1)
//			gocv.PutText(&imgDraw, i, image.Point{X: match.X, Y: match.Y + ySize}, gocv.FontHersheySimplex, 0.3, color.RGBA{G: 255, A: 255}, 1)
//		}
//		log.Printf("Results for %s: %v\n", i, matches)
//	}
//	gocv.IMWrite(pathDir+"founditems.png", imgDraw)
//
//	return map[string][]robotgo.Point{}
//}
