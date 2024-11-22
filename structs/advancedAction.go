package structs

import (
	"Dark-And-Darker/utils"
	"bytes"
	"fmt"
	"image"
	"image/color"
	"image/png"
	"log"
	"sort"
	"strings"
	"sync"

	"gocv.io/x/gocv"

	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
	"github.com/otiai10/gosseract/v2"
	//"github.com/vcaesar/bitmap"
)

type AdvancedActionInterface interface {
	ActionInterface

	GetName() string
	SetName(string)

	GetSubActions() []ActionInterface
	AddSubAction(ActionInterface)
	RemoveSubAction(ActionInterface, *widget.Tree)
	RenameActions(*widget.Tree)
}

type AdvancedAction struct {
	BaseAction                   //`json:"baseaction"`
	Name       string            `json:"name"`
	SubActions []ActionInterface `json:"subactions"`
}

func (a *AdvancedAction) GetSubActions() []ActionInterface {
	return a.SubActions
}

func (a *AdvancedAction) AddSubAction(action ActionInterface) {
	actionNum := len(a.GetSubActions()) + 1
	uid := fmt.Sprintf("%s.%d", a.GetUID(), actionNum)
	action.UpdateBaseAction(uid, a)

	a.SubActions = append(a.SubActions, action)
	log.Printf("Added new action: %s", action.String())
}

func (a *AdvancedAction) RemoveSubAction(action ActionInterface, tree *widget.Tree) {
	for i, c := range a.SubActions {
		if c == action {
			a.SubActions = append(a.SubActions[:i], a.SubActions[i+1:]...)
			log.Printf("Removing %s", action.GetUID())
			a.RenameActions(tree)
		}
	}
}

func (a *AdvancedAction) RenameActions(tree *widget.Tree) {
	for i, child := range a.SubActions {
		open := tree.IsBranchOpen(child.GetUID())
		child.SetUID(fmt.Sprintf("%s.%d", a.UID, i+1))
		if open {
			tree.OpenBranch(child.GetUID())
		}
		if n, ok := child.(AdvancedActionInterface); ok {
			n.RenameActions(tree)
		}
	}
}

func (a *AdvancedAction) Execute(ctx interface{}) error {
	log.Printf("Executing %s", a.Name)

	for _, c := range a.SubActions {
		c.Execute(ctx)
	}
	return nil
}
func (a *AdvancedAction) String() string { return "This is a Action with SubActions" }

//******************************************************************************************Loop

type LoopAction struct {
	Count          int `json:"loopcount"`
	AdvancedAction     //`json:"advancedaction"`
}

func (a *LoopAction) Execute(ctx interface{}) error {
	for i := 0; i < a.Count; i++ {
		fmt.Printf("Loop iteration %d\n", i+1)
		for _, action := range a.GetSubActions() {
			if err := action.Execute(ctx); err != nil {
				return err
			}
		}
	}
	return nil
}

func (a *LoopAction) String() string {
	return fmt.Sprintf("%s | %s%d", a.Name, utils.GetEmoji("Loop"), a.Count)
}

// ***************************************************************************************ImageSearch

type ImageSearchAction struct {
	Targets        []string  `json:"imagetargets"`
	SearchBox      SearchBox `json:"searchbox"`
	AdvancedAction           //`json:"advancedaction"`
}

func (a *ImageSearchAction) Execute(ctx interface{}) error {
	log.Printf("Image Search | %v in X1:%d Y1:%d X2:%d Y2:%d", a.Targets, a.SearchBox.LeftX, a.SearchBox.TopY, a.SearchBox.RightX, a.SearchBox.BottomY)
	w := a.SearchBox.RightX - a.SearchBox.LeftX
	h := a.SearchBox.BottomY - a.SearchBox.TopY

	captureImg :=robotgo.CaptureImg(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
	img, _  := gocv.ImageToMatRGB(captureImg)
	gocv.IMWrite("./images/search-area.png", img)
	imgDraw := img
	//defer imgDraw.Close()
	var xSplit, ySplit int
	if strings.Contains(a.SearchBox.Name, "Player") {
		xSplit = 5
		ySplit = 10
	} else if strings.Contains(a.SearchBox.Name, "Stash Inventory") {
		xSplit = 20
		ySplit = 12
	} else if strings.Contains(a.SearchBox.Name, "Merchant Inventory") {
		xSplit = 20
		ySplit = 12
	} else {
		xSplit = 1
		ySplit = 1
	}
	xSize := img.Cols() / ySplit
	ySize := img.Rows() / xSplit
	var splitAreas []image.Rectangle
	for r := 0; r < ySplit; r++ {
		for c := 0; c < xSplit; c++ {
			splitAreas = append(splitAreas, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
		}
	}

	var wg sync.WaitGroup
	results := make(map[string][]robotgo.Point)
	resultsMutex := &sync.Mutex{}
	for _, target := range a.Targets {
		wg.Add(1)
		go func(target string) {
			defer wg.Done()

			ip := "./images/icons/" + target + ".png"

			// Read the template
			template := gocv.IMRead(ip, gocv.IMReadColor)
			defer template.Close()
			if template.Empty() {
				fmt.Println("Error reading template image")
				return
			}

			var colorMatchwg sync.WaitGroup
			var matches []robotgo.Point
			var colorMatchResults []robotgo.Point
			colorMatchResultsMutex := &sync.Mutex{}
			emptyPoint := robotgo.Point{}

			for _, s := range splitAreas {
				colorMatchwg.Add(1)
				go func(s image.Rectangle) {
					defer colorMatchwg.Done()
					var point robotgo.Point
					point = checkHistogramMatch(img.Region(s), template, 0.15, target)
//					point := checkColorMatch(img.Region(s), template, 5)
//					log.Printf("slot: %v || at: %v", i, point)
					if point != emptyPoint {
						point = robotgo.Point{X: s.Min.X, Y: s.Min.Y}
						colorMatchResultsMutex.Lock()
						defer colorMatchResultsMutex.Unlock()
						colorMatchResults = append(colorMatchResults, point)
					}
				}(s)
			}
			colorMatchwg.Wait()
			matches = colorMatchResults
			// matches := findTemplateMatches(img, template, 0.93)

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
					match.X+xSize,
					match.Y+ySize,
				)
				gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 2)
				gocv.PutText(&imgDraw, target, image.Point{X: match.X, Y: match.Y+ySize}, gocv.FontHersheySimplex, 0.4, color.RGBA{G: 255, A: 255}, 1)
			}
			log.Printf("Results for %s: %v\n", target, matches)
		}(target)
		//draw rectangles around each match
	}
	wg.Wait()
	gocv.IMWrite("./images/founditems.png", imgDraw)
	//show temp window of matches surrounded by rectangles
	//	window := gocv.NewWindow("Matches")
	//    defer window.Close()
	//	window.IMShow(img)
	//    window.WaitKey(0)
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
func (a *ImageSearchAction) String() string {
	return fmt.Sprintf("%s Image Search for %d items in `%s` | %s", utils.GetEmoji("Image Search"), len(a.Targets), a.SearchBox.Name, a.Name)
}

// ***************************************************************************************OCR

type OcrAction struct {
	Target         string    `json:"texttarget"`
	SearchBox      SearchBox `json:"searchbox"`
	AdvancedAction           //`json:"advancedaction"`
}

func (a *OcrAction) Execute(ctx interface{}) error {
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

func (a *OcrAction) String() string {
	return fmt.Sprintf("%s OCR search for `%s` in `%s`", utils.GetEmoji("OCR"), a.Target, a.SearchBox.Name)
}

//******************************************************************************************Conditional

// type ConditionalAction struct {
// 	AdvancedAction
// 	Condition func(interface{}) bool
// }

// func (a *ConditionalAction) Execute(ctx interface{}) error {
// 	if a.Condition(ctx) {
// 		fmt.Println("Condition true. Executing subactions")
// 		for _, action := range a.SubActions {
// 			if err := action.Execute(ctx); err != nil {
// 				return err
// 			}
// 		}
// 	} else {
// 		fmt.Println("Condition false. Skipping block")
// 		// for _, action := range a.FalseActions {
// 		// 	if err := action.Execute(ctx); err != nil {
// 		// 		return err
// 		// 	}
// 		// }
// 	}
// 	return nil
// }

// func (a *ConditionalAction) String() string {
// 	return fmt.Sprintf("%sConditional | %s", utils.GetEmoji("Conditional"), a.Name)
// }

// func distance(p, other robotgo.Point) float64 {
//	dx := p.X - other.X
//	dy := p.Y - other.Y
//	return math.Sqrt(float64(dx*dx + dy*dy))
//}
//
//// filterClosePoints removes points that are within minDistance of any previous point
//func filterClosePoints(points []robotgo.Point, minDistance float64) []robotgo.Point {
//	if len(points) == 0 {
//		return points
//	}
//
//	// First point is always included
//	filtered := []robotgo.Point{points[0]}
//
//	// Check each point against all previously accepted points
//	for i := 1; i < len(points); i++ {
//		tooClose := false
//		for _, accepted := range filtered {
//			dist := distance(points[i], accepted)
//			log.Printf("distance: %f", dist)
//			if dist < minDistance {
//				tooClose = true
//				break
//			}
//		}
//		if !tooClose {
//			filtered = append(filtered, points[i])
//		}
//	}
//
//	return filtered
//}

func checkHistogramMatch(img, template gocv.Mat, tolerance float32, target string) robotgo.Point {
	labImg := gocv.NewMat()
	labTemplate := gocv.NewMat()
	defer labImg.Close()
	defer labTemplate.Close()
	gocv.CvtColor(img, &labImg, gocv.ColorBGRToLab)
	gocv.CvtColor(template, &labTemplate, gocv.ColorBGRToLab)
	lHistImg := gocv.NewMat()
	aHistImg := gocv.NewMat()
	blabHistImg := gocv.NewMat()
	defer lHistImg.Close()
	defer aHistImg.Close()
	defer blabHistImg.Close()

	lHistTemplate := gocv.NewMat()
	aHistTemplate := gocv.NewMat()
	blabHistTemplate := gocv.NewMat()
	defer lHistTemplate.Close()
	defer aHistTemplate.Close()
	defer blabHistTemplate.Close()

	labBins := 64
	gocv.CalcHist([]gocv.Mat{labImg}, []int{0}, gocv.NewMat(), &lHistImg, []int{labBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{labTemplate}, []int{0}, gocv.NewMat(), &lHistTemplate, []int{labBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{labImg}, []int{1}, gocv.NewMat(), &aHistImg, []int{labBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{labTemplate}, []int{1}, gocv.NewMat(), &aHistTemplate, []int{labBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{labImg}, []int{2}, gocv.NewMat(), &blabHistImg, []int{labBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{labTemplate}, []int{2}, gocv.NewMat(), &blabHistTemplate, []int{labBins}, []float64{0,256}, false)

	normType := gocv.NormMinMax
	gocv.Normalize(lHistImg, &lHistImg, 0, 1, normType)
	gocv.Normalize(aHistImg, &aHistImg, 0, 1, normType)
	gocv.Normalize(blabHistImg, &blabHistImg, 0, 1, normType)
	gocv.Normalize(lHistTemplate, &lHistTemplate, 0, 1, normType)
	gocv.Normalize(aHistTemplate, &aHistTemplate, 0, 1, normType)
	gocv.Normalize(blabHistTemplate, &blabHistTemplate, 0, 1, normType)
	compType := gocv.HistCmpBhattacharya
//	compType := gocv.HistCmpCorrel
	lSimilarity := gocv.CompareHist(lHistImg, lHistTemplate, compType)
	aSimilarity := gocv.CompareHist(aHistImg, aHistTemplate, compType)
	blabSimilarity := gocv.CompareHist(blabHistImg, blabHistTemplate, compType)


	hsvImg := gocv.NewMat()
	hsvTemplate := gocv.NewMat()
	defer hsvImg.Close()
	defer hsvTemplate.Close()
	gocv.CvtColor(img, &hsvImg, gocv.ColorBGRToHSV)
	gocv.CvtColor(template, &hsvTemplate, gocv.ColorBGRToHSV)
	hHistImg := gocv.NewMat()
	sHistImg := gocv.NewMat()
	vHistImg := gocv.NewMat()
	defer hHistImg.Close()
	defer sHistImg.Close()
	defer vHistImg.Close()

	hHistTemplate := gocv.NewMat()
	sHistTemplate := gocv.NewMat()
	vHistTemplate := gocv.NewMat()
	defer hHistTemplate.Close()
	defer sHistTemplate.Close()
	defer vHistTemplate.Close()

	hsvBins := 64
	gocv.CalcHist([]gocv.Mat{hsvImg}, []int{0}, gocv.NewMat(), &hHistImg, []int{hsvBins}, []float64{0,180}, false)
	gocv.CalcHist([]gocv.Mat{hsvTemplate}, []int{0}, gocv.NewMat(), &hHistTemplate, []int{hsvBins}, []float64{0,180}, false)
	gocv.CalcHist([]gocv.Mat{hsvImg}, []int{1}, gocv.NewMat(), &sHistImg, []int{hsvBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{hsvTemplate}, []int{1}, gocv.NewMat(), &sHistTemplate, []int{hsvBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{hsvImg}, []int{2}, gocv.NewMat(), &vHistImg, []int{hsvBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{hsvTemplate}, []int{2}, gocv.NewMat(), &vHistTemplate, []int{hsvBins}, []float64{0,256}, false)

	gocv.Normalize(hHistImg, &hHistImg, 0, 1, normType)
	gocv.Normalize(sHistImg, &sHistImg, 0, 1, normType)
	gocv.Normalize(vHistImg, &vHistImg, 0, 1, normType)
	gocv.Normalize(hHistTemplate, &hHistTemplate, 0, 1, normType)
	gocv.Normalize(sHistTemplate, &sHistTemplate, 0, 1, normType)
	gocv.Normalize(vHistTemplate, &vHistTemplate, 0, 1, normType)
	hSimilarity := gocv.CompareHist(hHistImg, hHistTemplate, compType)
	sSimilarity := gocv.CompareHist(sHistImg, sHistTemplate, compType)
	vSimilarity := gocv.CompareHist(vHistImg, vHistTemplate, compType)


	bHistImg := gocv.NewMat()
	gHistImg := gocv.NewMat()
	rHistImg := gocv.NewMat()
	defer bHistImg.Close()
	defer gHistImg.Close()
	defer rHistImg.Close()

	bHistTemplate := gocv.NewMat()
	gHistTemplate := gocv.NewMat()
	rHistTemplate := gocv.NewMat()
	defer bHistTemplate.Close()
	defer gHistTemplate.Close()
	defer rHistTemplate.Close()

	bgrBins := 64
	gocv.CalcHist([]gocv.Mat{img}, []int{0}, gocv.NewMat(), &bHistImg, []int{bgrBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{template}, []int{0}, gocv.NewMat(), &bHistTemplate, []int{bgrBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{img}, []int{1}, gocv.NewMat(), &gHistImg, []int{bgrBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{template}, []int{1}, gocv.NewMat(), &gHistTemplate, []int{bgrBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{img}, []int{2}, gocv.NewMat(), &rHistImg, []int{bgrBins}, []float64{0,256}, false)
	gocv.CalcHist([]gocv.Mat{template}, []int{2}, gocv.NewMat(), &rHistTemplate, []int{bgrBins}, []float64{0,256}, false)

	gocv.Normalize(bHistImg, &bHistImg, 0, 1, normType)
	gocv.Normalize(gHistImg, &gHistImg, 0, 1, normType)
	gocv.Normalize(rHistImg, &rHistImg, 0, 1, normType)
	gocv.Normalize(bHistTemplate, &bHistTemplate, 0, 1, normType)
	gocv.Normalize(gHistTemplate, &gHistTemplate, 0, 1, normType)
	gocv.Normalize(rHistTemplate, &rHistTemplate, 0, 1, normType)

	bSimilarity := gocv.CompareHist(bHistImg, bHistTemplate, compType)
	gSimilarity := gocv.CompareHist(gHistImg, gHistTemplate, compType)
	rSimilarity := gocv.CompareHist(rHistImg, rHistTemplate, compType)
//	log.Printf( "l: %v || a: %v || b: %v\n" +
//		"hue: %v || sat: %v || val: %v\n" +
//		"blue: %v || green: %v || red: %v",
//		lSimilarity, aSimilarity, blabSimilarity,
//		hSimilarity, sSimilarity, vSimilarity,
//		bSimilarity, gSimilarity, rSimilarity)
//		window := gocv.NewWindow("spot")
//	    defer window.Close()
//		window.IMShow(img)
//	    window.WaitKey(0)
//if ((bSimilarity > bgrTolerance && gSimilarity > bgrTolerance) ||
//	(bSimilarity > bgrTolerance && rSimilarity > bgrTolerance) ||
//	(gSimilarity > bgrTolerance && rSimilarity > bgrTolerance)) &&
//	((lSimilarity > tolerance && aSimilarity > tolerance) ||
//	(lSimilarity > tolerance && blabSimilarity > tolerance) ||
//	(blabSimilarity > tolerance && aSimilarity > tolerance)) {
//log.Printf( "l: %v || a: %v || b: %v\n" +
//	"hue: %v || sat: %v || val: %v\n" +
//	"blue: %v || green: %v || red: %v",
//	lSimilarity, aSimilarity, blabSimilarity,
//	hSimilarity, sSimilarity, vSimilarity,
//	bSimilarity, gSimilarity, rSimilarity)
if bSimilarity < tolerance &&
	gSimilarity < tolerance &&
	rSimilarity < tolerance &&
	hSimilarity < tolerance &&
	sSimilarity < tolerance &&
	vSimilarity < tolerance &&
	lSimilarity < tolerance &&
	aSimilarity < tolerance &&
	blabSimilarity < tolerance {
	log.Printf( "target: %v\n" +
		"l: %.4f || a: %.4f || b: %.4f\n" +
		"hue: %.4f || sat: %.4f || val: %.4f\n" +
		"blue: %.4f || green: %.4f || red: %.4f",
		target,
		lSimilarity, aSimilarity, blabSimilarity,
		hSimilarity, sSimilarity, vSimilarity,
		bSimilarity, gSimilarity, rSimilarity)
		return robotgo.Point{X: img.Size()[0], Y: img.Size()[1]}
	} else {
		return robotgo.Point{}
	}
}

func checkColorMatch(img, template gocv.Mat, threshold int8) robotgo.Point {
	var templateBGRAvgs, templateBGRMeds, templateHSVAvgs, templateHSVMeds = [3]uint8{}, [3]uint8{}, [3]uint8{}, [3]uint8{}
	var templateLABAvgs, templateLABMeds = [3]int8{}, [3]int8{}
	templateBGRAvgs, templateBGRMeds = extractValuesBGR(template)
	templateHSVAvgs, templateHSVMeds = extractValuesHSV(template)
	templateLABAvgs, templateLABMeds = extractValuesLAB(template)

	var imgBGRAvgs, imgBGRMeds, imgHSVAvgs, imgHSVMeds = [3]uint8{}, [3]uint8{}, [3]uint8{}, [3]uint8{}
	var imgLABAvgs, imgLABMeds = [3]int8{}, [3]int8{}
	imgBGRAvgs, imgBGRMeds = extractValuesBGR(img)
	imgHSVAvgs, imgHSVMeds = extractValuesHSV(img)
	imgLABAvgs, imgLABMeds = extractValuesLAB(img)

	switch [3]uint8{}{
	case imgBGRMeds:
	case imgHSVMeds:
	case templateBGRMeds:
	case templateHSVMeds:
		}
	switch [3]int8{}{
	case imgLABMeds:
	case templateLABMeds:

		}
		if (areWithinToleranceInt(imgLABAvgs, templateLABAvgs, threshold) &&
			areWithinToleranceSingleChannel(imgHSVAvgs[0], templateHSVAvgs[0], uint8(threshold)) &&
			areWithinToleranceSingleChannel(imgHSVAvgs[2], templateHSVAvgs[2], uint8(threshold))) && //||
			((areWithinToleranceInt(imgLABAvgs, templateLABAvgs, threshold)) &&
			areWithinToleranceAllChannels(imgBGRAvgs, templateBGRAvgs, uint8(threshold))) && //||
			(areWithinToleranceAllChannels(imgBGRAvgs, templateBGRAvgs, uint8(threshold)) &&
			areWithinToleranceSingleChannel(imgHSVAvgs[0], templateHSVAvgs[0], uint8(threshold)) &&
				areWithinToleranceSingleChannel(imgHSVAvgs[2], templateHSVAvgs[2], uint8(threshold))) {
			log.Printf("---------------------------------------SUCCESS----------------------------------------------\n" +
							"BGR averages: %v, %v ||| LAB averages: %v, %v ||| HSV averages: %v, %v\n" +
							"BGR medians: %v, %v ||| LAB medians: %v, %v ||| HSV medians: %v, %v",
							imgBGRAvgs, templateBGRAvgs, imgLABAvgs, templateLABAvgs,  imgHSVAvgs, templateHSVAvgs,
							imgBGRMeds, templateBGRMeds, imgLABMeds, templateLABMeds,  imgHSVAvgs, templateHSVMeds)


		return robotgo.Point{X: img.Size()[0], Y: img.Size()[1]}
	} else {
		log.Printf("------------FAILURE-----------\n" +
			"BGR averages: %v, %v ||| LAB averages: %v, %v ||| HSV averages: %v, %v\n" +
			"BGR medians: %v, %v ||| LAB medians: %v, %v ||| HSV medians: %v, %v",
			imgBGRAvgs, templateBGRAvgs, imgLABAvgs, templateLABAvgs,  imgHSVAvgs, templateHSVAvgs,
			imgBGRMeds, templateBGRMeds, imgLABMeds, templateLABMeds,  imgHSVAvgs, templateHSVMeds)
		return robotgo.Point{}
	}
}
func areWithinToleranceSingleChannel(img, template uint8, tolerance uint8) bool {
        if img < (template-tolerance) || img > (template+tolerance) {
            return false
        }
    return true
}

func areWithinToleranceAllChannels(img, template [3]uint8, tolerance uint8) bool {
    for i := 0; i < 3; i++ {
		lowerBound := template[i] - tolerance
        upperBound := template[i] + tolerance
//        log.Printf("Comparing img[%d] = %d to template[%d] = %d with tolerance %d: Range [%d, %d]\n",
//            i, img[i], i, template[i], tolerance, lowerBound, upperBound)

        if img[i] < lowerBound || img[i] > upperBound {
//            log.Printf("Out of range: img[%d] = %d is not within [%d, %d]\n", i, img[i], lowerBound, upperBound)
            return false
        }
		//        if img[i] < (template[i]-tolerance) || img[i] > (template[i]+tolerance) {
//            return false
//        }
    }
    return true
}
func areWithinToleranceInt(img, template [3]int8, tolerance int8) bool {
    for i := 0; i < 3; i++ {
		var lowerBound, upperBound int8
//		if  -128 + tolerance >= template[i] {
//			lowerBound = -128
//		} else{
			lowerBound = template[i] - tolerance
//		}
//
//		if 127 - tolerance <= template[i] {
//			upperBound = 127
//		} else {
        	upperBound = template[i] + tolerance
//		}
//        log.Printf("Comparing img[%d] = %d to template[%d] = %d with tolerance %d: Range [%d, %d]\n",
//            i, img[i], i, template[i], tolerance, lowerBound, upperBound)
		if lowerBound > upperBound {
			if img[i] > lowerBound || img[i] < upperBound {
		//            log.Printf("Out of range: img[%d] = %d is not within [%d, %d]\n", i, img[i], lowerBound, upperBound)
	            return false
	        }
		}else {
	        if img[i] < lowerBound || img[i] > upperBound {
	//            log.Printf("Out of range: img[%d] = %d is not within [%d, %d]\n", i, img[i], lowerBound, upperBound)
	            return false
	        }
		}

		//        if img[i] < template[i]-tolerance || img[i] > template[i]+tolerance{
//            return false
//        }
    }
    return true
}

func extractValuesBGR(img gocv.Mat) ([3]uint8, [3]uint8) {
	split := gocv.Split(img)
	b, g, r := split[0], split[1], split[2]
	defer b.Close()
	defer g.Close()
	defer r.Close()

	avgB := uint8(b.Mean().Val1)
	avgG := uint8(g.Mean().Val1)
	avgR := uint8(r.Mean().Val1)
//	avgB := int(img.Mean().Val1)
//	avgG := int(img.Mean().Val2)
//	avgR := int(img.Mean().Val3)

	// Convert the channels to slices to easily calculate the median
    blueData := getChannelData(b)
    greenData := getChannelData(g)
    redData := getChannelData(r)

    // Calculate the median of each channel
	medB := calculateMedian(blueData)
    medG := calculateMedian(greenData)
    medR := calculateMedian(redData)
	return [3]uint8{avgB, avgG, avgR}, [3]uint8{medB, medG, medR}
}

func extractValuesHSV(img gocv.Mat) ([3]uint8, [3]uint8) {
	hsvMat := gocv.NewMat()
	defer hsvMat.Close()
	//convert img to HSV
	gocv.CvtColor(img, &hsvMat, gocv.ColorBGRToHSV)
	split := gocv.Split(hsvMat)
	h, s, v := split[0], split[1], split[2]
	defer h.Close()
	defer s.Close()
	defer v.Close()

	avgH := uint8(h.Mean().Val1)
	avgS := uint8(s.Mean().Val1)
	avgV := uint8(v.Mean().Val1)

    hueData := getChannelData(h)
    satData := getChannelData(s)
    valData := getChannelData(v)

    // Calculate the median of each channel
	medH := calculateMedian(hueData)
    medS := calculateMedian(satData)
    medV := calculateMedian(valData)

	return [3]uint8{avgH, avgS, avgV}, [3]uint8{medH, medS, medV}
}
func extractValuesLAB(img gocv.Mat) ([3]int8, [3]int8) {
	labMat := gocv.NewMat()
	defer labMat.Close()
	gocv.CvtColor(img, &labMat, gocv.ColorBGRToLab)
	split := gocv.Split(labMat)
	l, a, b := split[0], split[1], split[2]
	defer l.Close()
	defer a.Close()
	defer b.Close()
	avgL := int8(l.Mean().Val1)
	avgA := int8(a.Mean().Val1)
	avgB := int8(b.Mean().Val1)

	// Convert the channels to slices to easily calculate the median
    lData := getChannelData(l)
    aData := getChannelData(a)
    bData := getChannelData(b)

    // Calculate the median of each channel
	medL := calculateMedian(lData)
	medA := calculateMedian(aData)
	medB := calculateMedian(bData)
	return [3]int8{avgL, avgA, avgB}, [3]int8{int8(medL), int8(medA), int8(medB)}
}
//func extractValuesLCH(img gocv.Mat) ([3]uint8, [3]uint8) {
//	split := gocv.Split(img)
//	l, c, h := split[0], split[1], split[2]
//	defer l.Close()
//	defer c.Close()
//	defer h.Close()
//
//	avgL := uint8(l.Mean().Val1)
//	avgC := uint8(c.Mean().Val1)
//	avgH := uint8(h.Mean().Val1)
////	avgL := int(img.Mean().Val1)
////	avgC := int(img.Mean().Val2)
////	avgH := int(img.Mean().Val3)
//
//	// Convert the channels to slices to easily calculate the median
//    blueData := getChannelData(l)
//    greenData := getChannelData(c)
//    redData := getChannelData(h)
//
//    // Calculate the median of each channel
//	medB := calculateMedian(blueData)
//    medG := calculateMedian(greenData)
//    medR := calculateMedian(redData)
//	return [3]uint8{avgL, avgC, avgH}, [3]uint8{medB, medG, medR}
//}

// converts a gocv Mat to a slice of uint8 for median calculation.
func getChannelData(channel gocv.Mat) []uint8 {
    rows := channel.Rows()
    cols := channel.Cols()
    data := make([]uint8, rows*cols)

    data, _ = channel.DataPtrUint8()

	return data
}
func calculateMedian(data []uint8) uint8 {
    // Sort the data slice
    sort.Slice(data, func(i, j int) bool {
        return data[i] < data[j]
    })

    // Calculate median
    mid := len(data) / 2
    if len(data)%2 == 0 {
        return (data[mid-1] + data[mid]) / 2
    }
    return data[mid]
}

func findTemplateMatches(img, template gocv.Mat, threshold float32) []robotgo.Point {
	// Create the result matrix
	result := gocv.NewMat()
	defer result.Close()

	// Perform template matching
	mask := gocv.NewMat()
	defer mask.Close()

	region := template.Region(image.Rect(0, 0, 25, 25))

	//method 5 works best
	gocv.MatchTemplate(img, region, &result, 5, mask)

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

func findFeatureMatches(img, template gocv.Mat) []Match {
	orbTemplate := gocv.NewORBWithParams(20,
		1.2,
		8,
		10,
		0,
		2,
		gocv.ORBScoreTypeHarris,
		10,
		20,
	)
	defer orbTemplate.Close()
	orbSource := gocv.NewORBWithParams(10000,
		1.1,
		12,
		10,
		0,
		2,
		gocv.ORBScoreTypeHarris,
		10,
		5,
	)
	defer orbSource.Close()

	kp1, desc1 := orbTemplate.DetectAndCompute(template, gocv.NewMat())
	kp2, desc2 := orbSource.DetectAndCompute(img, gocv.NewMat())
	defer desc1.Close()
	defer desc2.Close()

	matcher := gocv.NewBFMatcher()
	defer matcher.Close()

	matches := matcher.Match(desc1, desc2)

	log.Printf("%v", matches)

	templateKPs := gocv.NewMat()
	gocv.DrawKeyPoints(template, kp1, &templateKPs, color.RGBA{G: 255}, 0)

	templateKPsWin := gocv.NewWindow("templateKPsWin")
	defer templateKPsWin.Close()
	templateKPsWin.IMShow(templateKPs)
	gocv.WaitKey(0)

	imgKPs := gocv.NewMat()
	gocv.DrawKeyPoints(img, kp2, &imgKPs, color.RGBA{G: 255}, 0)

	imgKPsWin := gocv.NewWindow("imgKPsWin")
	defer imgKPsWin.Close()
	imgKPsWin.IMShow(imgKPs)
	gocv.WaitKey(0)

	matchesImg := gocv.NewMat()
	gocv.DrawMatches(template, kp1, img, kp2, matches, &matchesImg, color.RGBA{R: 255}, color.RGBA{G: 255}, nil, 2)

	matchesImgWin := gocv.NewWindow("matchesImgWin")
	defer matchesImgWin.Close()
	matchesImgWin.IMShow(matchesImg)
	gocv.WaitKey(0)

	return []Match{}
}

// Match represents a detected match in the image
type Match struct {
	Location image.Point
	Size     image.Point
	Score    float64
}

//// transformPoints transforms a set of points using a homography matrix
//func transformPoints(points []image.Point, H gocv.Mat) []image.Point {
//    var transformed []image.Point
//    for _, pt := range points {
//        // Create 3x1 matrix for point
//        srcMat := gocv.NewMatWithSize(3, 1, gocv.MatTypeCV64F)
//        defer srcMat.Close()
//        srcMat.SetDoubleAt(0, 0, float64(pt.X))
//        srcMat.SetDoubleAt(1, 0, float64(pt.Y))
//        srcMat.SetDoubleAt(2, 0, 1.0)
//
//        // Apply homography
//        dstMat := gocv.NewMat()
//        defer dstMat.Close()
//        gocv.GemmWithParams(H, srcMat, 1.0, gocv.NewMat(), 0.0, &dstMat, 0)
//
//        // Convert back to point
//        w := dstMat.GetDoubleAt(2, 0)
//        x := int(dstMat.GetDoubleAt(0, 0) / w)
//        y := int(dstMat.GetDoubleAt(1, 0) / w)
//
//        transformed = append(transformed, image.Point{X: x, Y: y})
//    }
//    return transformed
//}
