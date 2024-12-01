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

	"github.com/go-vgo/robotgo"
	"github.com/otiai10/gosseract/v2"
)

type AdvancedActionInterface interface {
	ActionInterface

	GetName() string
	SetName(string)

	GetSubActions() []ActionInterface
	AddSubAction(ActionInterface)
	RemoveSubAction(ActionInterface)
	RenameActions()
}

type AdvancedAction struct {
	BaseAction                   //`json:"baseaction"`
	Name       string            `json:"name"`
	SubActions []ActionInterface `json:"subactions"`
}

func NewAdvancedAction(name string, subActions []ActionInterface) *AdvancedAction {
	return &AdvancedAction{
		BaseAction: NewBaseAction(),
		Name:       name,
		SubActions: subActions,
	}
}

func (a *AdvancedAction) GetSubActions() []ActionInterface {
	return a.SubActions
}

func (a *AdvancedAction) AddSubAction(action ActionInterface) {
	actionNum := len(a.GetSubActions()) + 1
	uid := fmt.Sprintf("%s.%d", a.GetUID(), actionNum)
	action.UpdateBaseAction(uid, a)

	a.SubActions = append(a.SubActions, action)
	log.Printf("Added new action: %v %s", uid, action.String())
}

func (a *AdvancedAction) RemoveSubAction(action ActionInterface) {
	for i, c := range a.SubActions {
		if c == action {
			a.SubActions = append(a.SubActions[:i], a.SubActions[i+1:]...)
			log.Printf("Removing %s", action.GetUID())
			a.RenameActions()
		}
	}
}

func (a *AdvancedAction) RenameActions() {
	for i, child := range a.SubActions {
		if n, ok := child.(AdvancedActionInterface); ok {
			n.RenameActions()
		}
		//		open := tree.IsBranchOpen(child.GetUID())
		child.SetUID(fmt.Sprintf("%s.%d", a.UID, i+1))
		//		if open {
		//			tree.OpenBranch(child.GetUID())
		//		}
		//		if n, ok := child.(AdvancedActionInterface); ok {
		//			n.RenameActions(tree)
		//		}
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

func NewLoopAction(count int, name string, subActions []ActionInterface) *LoopAction {
	return &LoopAction{
		AdvancedAction: *NewAdvancedAction(name, subActions),
		Count:          count,
	}
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

func NewImageSearchAction(name string, subActions []ActionInterface, targets []string, searchbox SearchBox) *ImageSearchAction {
	return &ImageSearchAction{
		AdvancedAction: *NewAdvancedAction(name, subActions),
		Targets:        targets,
		SearchBox:      searchbox,
	}
}

func (a *ImageSearchAction) Execute(ctx interface{}) error {
	log.Printf("Image Search | %v in X1:%d Y1:%d X2:%d Y2:%d", a.Targets, a.SearchBox.LeftX, a.SearchBox.TopY, a.SearchBox.RightX, a.SearchBox.BottomY)
	w := a.SearchBox.RightX - a.SearchBox.LeftX
	h := a.SearchBox.BottomY - a.SearchBox.TopY

	captureImg := robotgo.CaptureImg(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
	img, _ := gocv.ImageToMatRGB(captureImg)
	gocv.IMWrite("./images/search-area.png", img)
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

			ip := "./images/icons/" + target + ".png"

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
	gocv.IMWrite("./images/founditems.png", imgDraw)
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

func removeDupes() {
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
	//	})
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
