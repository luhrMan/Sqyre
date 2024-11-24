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
//	w := a.SearchBox.RightX - a.SearchBox.LeftX
//	h := a.SearchBox.BottomY - a.SearchBox.TopY
//
//	captureImg :=robotgo.CaptureImg(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
//	img, _  := gocv.ImageToMatRGB(captureImg)
//	gocv.IMWrite("./images/search-area.png", img)
	img := gocv.IMRead("./images/stash-area.png", gocv.IMReadColor)
	defer img.Close()
//	imgDraw := gocv.NewMat()
	imgDraw := img.Clone()
	defer imgDraw.Close()
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
	var tolerance float32 = 0.05
	if strings.Contains(a.SearchBox.Name, "Stash") {
		tolerance = 0.1
	} else if strings.Contains(a.SearchBox.Name, "Merchant"){
		tolerance = 0.15
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
			templateCut := template.Region(image.Rect(borderSize, borderSize, template.Cols()-borderSize, template.Rows()-borderSize)) //template.Size()[0]-borderSize, template.Size()[1]-borderSize))

			var colorMatchwg sync.WaitGroup
			var matches []robotgo.Point
//			var colorMatchResults []robotgo.Point
			colorMatchResultsMutex := &sync.Mutex{}
			emptyPoint := robotgo.Point{}

			for _, s := range splitAreas {
				colorMatchwg.Add(1)
				go func(s image.Rectangle) {
					defer colorMatchwg.Done()
					var point robotgo.Point
					point = checkHistogramMatch(img.Region(s), templateCut, tolerance, target)
//					point := checkColorMatch(img.Region(s), template, 5)
//					log.Printf("slot: %v || at: %v", i, point)
					if point != emptyPoint {
						point = robotgo.Point{X: s.Min.X, Y: s.Min.Y}
						colorMatchResultsMutex.Lock()
						matches = append(matches, point)
						defer colorMatchResultsMutex.Unlock()
					}
				}(s)
			}
			colorMatchwg.Wait()
//			matches = colorMatchResults
//			 matches := findTemplateMatches(img, templateCut, 0.95)
			sort.Slice(matches, func(i, j int) bool {
				return matches[i].Y < matches[j].Y
			})

			resultsMutex.Lock()
			defer resultsMutex.Unlock()
			results[target] = matches
		}(target)
		//draw rectangles around each match
	}
	wg.Wait()
//	removeDupes := make(map[robotgo.Point]bool)
//	for _, matches := range results {
//		k := 0
//		for j := range matches {
//			if !removeDupes[matches[j]] {
//				removeDupes[matches[j]] = true
//				matches[k] = matches[j]
//				k++
//			}
//		}
//	}
	for i, matches := range results {
		for _, match := range matches {
			rect := image.Rect(
				match.X,
				match.Y,
				match.X+xSize-(borderSize*2),
				match.Y+ySize-(borderSize*2),
				)
			gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 1)
			gocv.PutText(&imgDraw, i, image.Point{X: match.X, Y: match.Y+ySize}, gocv.FontHersheySimplex, 0.3, color.RGBA{G: 255, A: 255}, 1)
		}
		log.Printf("Results for %s: %v\n", i, matches)
	}

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
	normType := gocv.NormMinMax
	compType := gocv.HistCmpBhattacharya

	getColorChannels := func(image gocv.Mat, colorModel gocv.ColorConversionCode) gocv.Mat{
		colors := gocv.NewMat()
		gocv.CvtColor(image, &colors, colorModel)
		return colors
	}

	calculateColorModelSimilarities := func(img1, img2 gocv.Mat, bins int) []float32 {
		comparisons := []float32{0, 0, 0}
		img1Channels := gocv.Split(img1)
		img2Channels := gocv.Split(img2)
		for c := range img1Channels {
			gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0,256}, false)
			gocv.Normalize(img1Channels[c], &img1Channels[c], 0, 1, normType)
		}
		for c := range img2Channels {
			gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0,256}, false)
			gocv.Normalize(img2Channels[c], &img2Channels[c], 0, 1, normType)
			comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType)
		}
		return comparisons
	}

	imgGray := getColorChannels(img, gocv.ColorBGRToGray)
	templateGray := getColorChannels(template, gocv.ColorBGRToGray)
	simGray := calculateColorModelSimilarities(imgGray, templateGray, 64)

	imgLAB := getColorChannels(img, gocv.ColorBGRToLab)
	templateLAB := getColorChannels(template, gocv.ColorBGRToLab)
	simLAB := calculateColorModelSimilarities(imgLAB, templateLAB, 64)

	simBGR := calculateColorModelSimilarities(img, template, 64)

//	grayImg := gocv.NewMat()
//	grayTemplate := gocv.NewMat()
//	grayHistImg := gocv.NewMat()
//	grayHistTemplate := gocv.NewMat()
//	defer grayImg.Close()
//	defer grayTemplate.Close()
//	defer grayHistImg.Close()
//	defer grayHistTemplate.Close()
//
//	gocv.CvtColor(img, &grayImg, gocv.ColorBGRToGray)
//	gocv.CvtColor(template, &grayTemplate, gocv.ColorBGRToGray)
//
//	grayBins := 64
//	gocv.CalcHist([]gocv.Mat{grayImg}, []int{0}, gocv.NewMat(), &grayHistImg, []int{grayBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{grayTemplate}, []int{0}, gocv.NewMat(), &grayHistTemplate, []int{grayBins}, []float64{0,256}, false)
//	gocv.Normalize(grayHistImg, &grayHistImg, 0, 1, normType)
//	gocv.Normalize(grayHistTemplate, &grayHistTemplate, 0, 1, normType)
//	graySimilarity := gocv.CompareHist(grayHistImg, grayHistTemplate, compType)


//	graySimilarity := 3
//	var bgrTolerance float32 = 0.15
//	labImg := gocv.NewMat()
//	labTemplate := gocv.NewMat()
//	lHistImg := gocv.NewMat()
//	aHistImg := gocv.NewMat()
//	blabHistImg := gocv.NewMat()
//	lHistTemplate := gocv.NewMat()
//	aHistTemplate := gocv.NewMat()
//	blabHistTemplate := gocv.NewMat()
//	defer labImg.Close()
//	defer labTemplate.Close()
//	defer lHistImg.Close()
//	defer aHistImg.Close()
//	defer blabHistImg.Close()
//	defer lHistTemplate.Close()
//	defer aHistTemplate.Close()
//	defer blabHistTemplate.Close()
//
//	gocv.CvtColor(img, &labImg, gocv.ColorBGRToLab)
//	gocv.CvtColor(template, &labTemplate, gocv.ColorBGRToLab)
//
//
//	labBins := 64
//	gocv.CalcHist([]gocv.Mat{labImg}, []int{0}, gocv.NewMat(), &lHistImg, []int{labBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{labTemplate}, []int{0}, gocv.NewMat(), &lHistTemplate, []int{labBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{labImg}, []int{1}, gocv.NewMat(), &aHistImg, []int{labBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{labTemplate}, []int{1}, gocv.NewMat(), &aHistTemplate, []int{labBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{labImg}, []int{2}, gocv.NewMat(), &blabHistImg, []int{labBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{labTemplate}, []int{2}, gocv.NewMat(), &blabHistTemplate, []int{labBins}, []float64{0,256}, false)
//
//	gocv.Normalize(lHistImg, &lHistImg, 0, 1, normType)
//	gocv.Normalize(aHistImg, &aHistImg, 0, 1, normType)
//	gocv.Normalize(blabHistImg, &blabHistImg, 0, 1, normType)
//	gocv.Normalize(lHistTemplate, &lHistTemplate, 0, 1, normType)
//	gocv.Normalize(aHistTemplate, &aHistTemplate, 0, 1, normType)
//	gocv.Normalize(blabHistTemplate, &blabHistTemplate, 0, 1, normType)
//	compType := gocv.HistCmpCorrel
//	lSimilarity := gocv.CompareHist(lHistImg, lHistTemplate, compType)
//	aSimilarity := gocv.CompareHist(aHistImg, aHistTemplate, compType)
//	blabSimilarity := gocv.CompareHist(blabHistImg, blabHistTemplate, compType)

	// ------------------------------------------------------------------------HSV
	hsvImg := gocv.NewMat()
	hsvTemplate := gocv.NewMat()
	hHistImg := gocv.NewMat()
	sHistImg := gocv.NewMat()
	vHistImg := gocv.NewMat()
	hHistTemplate := gocv.NewMat()
	sHistTemplate := gocv.NewMat()
	vHistTemplate := gocv.NewMat()
	defer hsvImg.Close()
	defer hsvTemplate.Close()
	defer hHistImg.Close()
	defer sHistImg.Close()
	defer vHistImg.Close()
	defer hHistTemplate.Close()
	defer sHistTemplate.Close()
	defer vHistTemplate.Close()
	gocv.CvtColor(img, &hsvImg, gocv.ColorBGRToHSV)
	gocv.CvtColor(template, &hsvTemplate, gocv.ColorBGRToHSV)

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
	sSimilarity := gocv.CompareHist(sHistImg, sHistTemplate, compType) / 2
	vSimilarity := gocv.CompareHist(vHistImg, vHistTemplate, compType)

	// ------------------------------------------------------------------------BGR
//	bHistImg := gocv.NewMat()
//	gHistImg := gocv.NewMat()
//	rHistImg := gocv.NewMat()
//	bHistTemplate := gocv.NewMat()
//	gHistTemplate := gocv.NewMat()
//	rHistTemplate := gocv.NewMat()
//	defer bHistImg.Close()
//	defer gHistImg.Close()
//	defer rHistImg.Close()
//
//	defer bHistTemplate.Close()
//	defer gHistTemplate.Close()
//	defer rHistTemplate.Close()
//
//	bgrBins := 64
//	gocv.CalcHist([]gocv.Mat{img}, []int{0}, gocv.NewMat(), &bHistImg, []int{bgrBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{template}, []int{0}, gocv.NewMat(), &bHistTemplate, []int{bgrBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{img}, []int{1}, gocv.NewMat(), &gHistImg, []int{bgrBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{template}, []int{1}, gocv.NewMat(), &gHistTemplate, []int{bgrBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{img}, []int{2}, gocv.NewMat(), &rHistImg, []int{bgrBins}, []float64{0,256}, false)
//	gocv.CalcHist([]gocv.Mat{template}, []int{2}, gocv.NewMat(), &rHistTemplate, []int{bgrBins}, []float64{0,256}, false)
//
//	gocv.Normalize(bHistImg, &bHistImg, 0, 1, normType)
//	gocv.Normalize(gHistImg, &gHistImg, 0, 1, normType)
//	gocv.Normalize(rHistImg, &rHistImg, 0, 1, normType)
//	gocv.Normalize(bHistTemplate, &bHistTemplate, 0, 1, normType)
//	gocv.Normalize(gHistTemplate, &gHistTemplate, 0, 1, normType)
//	gocv.Normalize(rHistTemplate, &rHistTemplate, 0, 1, normType)
//
//	bSimilarity := gocv.CompareHist(bHistImg, bHistTemplate, compType)
//	gSimilarity := gocv.CompareHist(gHistImg, gHistTemplate, compType)
//	rSimilarity := gocv.CompareHist(rHistImg, rHistTemplate, compType)
//	log.Printf( "target: %v gray: %.4f\n" +
//		"l: %.4f || a: %.4f || b: %.4f\n" +
//		"hue: %.4f || sat: %.4f || val: %.4f\n" +
//		"blue: %.4f || green: %.4f || red: %.4f",
//		target, graySimilarity,
//		lSimilarity, aSimilarity, blabSimilarity,
//		hSimilarity, sSimilarity, vSimilarity,
//		bSimilarity, gSimilarity, rSimilarity)
	if
	//	simGray < 0.06 &&
		simBGR[0] < tolerance &&
		simBGR[1] < tolerance &&
		simBGR[2] < tolerance &&
		hSimilarity < tolerance &&
		sSimilarity < tolerance &&
		vSimilarity < tolerance &&
		simLAB[0] < tolerance &&
		simLAB[1] < tolerance &&
		simLAB[2] < tolerance {

//	lSimilarity < tolerance &&
//	aSimilarity <  tolerance &&
//	blabSimilarity < tolerance {
	log.Printf( "target: %v gray: %.4f\n" +
		"l: %.4f || a: %.4f || b: %.4f\n" +
		"hue: %.4f || sat: %.4f || val: %.4f\n" +
		"blue: %.4f || green: %.4f || red: %.4f",
		target, simGray[0],
		simLAB[0], simLAB[1], simLAB[2],
		hSimilarity, sSimilarity, vSimilarity,
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