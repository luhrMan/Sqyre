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

	capture := robotgo.CaptureScreen(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
	robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/search-area.jpeg")
	// capture := robotgo.CaptureScreen(a.SearchBox.LeftX, a.SearchBox.TopY, w, h)
	defer robotgo.FreeBitmap(capture)

	img := gocv.IMRead("./images/search-area.jpeg", gocv.IMReadColor)
	defer img.Close()
	if img.Empty() {
		fmt.Println("Error reading main image")
	}

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

			xSize := img.Cols() / ySplit
			ySize := img.Rows() / xSplit
			var splitAreas []image.Rectangle
			for r := 0; r < ySplit; r++ {
				for c := 0; c < xSplit; c++ {
					splitAreas = append(splitAreas, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
				}
			}
			var matches []robotgo.Point
			emptyPoint := robotgo.Point{0,0}
			for i, s := range splitAreas {
				log.Println(i)
				point := checkColorMatch(img.Region(s), template, 3, xSplit, ySplit)
				if point != emptyPoint {
					matches = append(matches, point)
				}
			}

			// matches := findTemplateMatches(img, template, 0.93)

			sort.Slice(matches, func(i, j int) bool {
				return matches[i].Y < matches[j].Y
			})

			imgDraw := img
			//draw rectangles around each match
			for _, match := range matches {
				rect := image.Rect(
					match.X,
					match.Y,
					match.X+template.Cols(),
					match.Y+template.Rows(),
				)
				gocv.Rectangle(&imgDraw, rect, color.RGBA{R: 255, A: 255}, 2)
			}
			gocv.IMWrite("./images/founditems.jpeg", imgDraw)

			resultsMutex.Lock()
			results[target] = matches
			resultsMutex.Unlock()

			log.Printf("Results for %s: %v\n", target, matches)
		}(target)
	}
	wg.Wait()
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

func checkColorMatch(img, template gocv.Mat, threshold, xSplit, ySplit int) robotgo.Point {
	var templateBGRAvgs, templateBGRMeds = [3]int{}, [3]int{}
	templateBGRAvgs, templateBGRMeds = extractValuesBGR(template)
	var templateHSVAvgs, templateHSVMeds = [3]int{}, [3]int{}
	templateHSVAvgs, templateHSVMeds = extractValuesHSV(template)

	var imgBGRAvgs, imgBGRMeds, imgHSVAvgs, imgHSVMeds = [3]int{}, [3]int{}, [3]int{}, [3]int{}
	// imgBGRAvgs, imgBGRMeds = extractValuesBGR(img)
	// imgHSVAvgs, imgHSVMeds = extractValuesHSV(img)
	// log.Print("imgBGRAvgs, imgBGRMeds, imgHSVAvgs, imgHSVMeds, templateBGRAvgs, templateBGRMeds, templateHSVAvgs, templateHSVMeds")
	// log.Print("", imgBGRAvgs, imgBGRMeds, imgHSVAvgs, imgHSVMeds, templateBGRAvgs, templateBGRMeds, templateHSVAvgs, templateHSVMeds)

	abs := func(x int) int {
		if x < 0 {
			return -x
		}
		return x
	}

	imgBGRAvgs, imgBGRMeds = extractValuesBGR(img)
	imgHSVAvgs, imgHSVMeds = extractValuesHSV(img)
	if abs(templateBGRAvgs[0]-imgBGRAvgs[0]) <= threshold &&
		abs(templateBGRAvgs[1]-imgBGRAvgs[1]) <= threshold &&
		abs(templateBGRAvgs[2]-imgBGRAvgs[2]) <= threshold &&
		abs(templateBGRMeds[0]-imgBGRMeds[0]) <= threshold &&
		abs(templateBGRMeds[1]-imgBGRMeds[1]) <= threshold &&
		abs(templateBGRMeds[2]-imgBGRMeds[2]) <= threshold {
		// abs(templateHSVAvgs[0]-imgHSVAvgs[0]) <= threshold &&
		// abs(templateHSVAvgs[1]-imgHSVAvgs[1]) <= threshold &&
		// abs(templateHSVAvgs[2]-imgHSVAvgs[2]) <= threshold &&
		// abs(templateHSVMeds[0]-imgHSVMeds[0]) <= threshold &&
		// abs(templateHSVMeds[1]-imgHSVMeds[1]) <= threshold &&
		// abs(templateHSVMeds[2]-imgHSVMeds[2]) <= threshold {
		log.Println("BGR averages:")
		log.Print(imgBGRAvgs, templateBGRAvgs)
		log.Println("BGR medians")
		log.Print(imgBGRMeds, templateBGRMeds)
		log.Println("HSV averages")
		log.Print(imgHSVAvgs, templateHSVAvgs)
		log.Println("HSV medians")
		log.Print(imgHSVMeds, templateHSVMeds)
		return robotgo.Point{X: img.Size()[0], Y: img.Size()[1]}
	}
	return robotgo.Point{0, 0}
}

func extractValuesBGR(img gocv.Mat) ([3]int, [3]int) {
	avgB := int(img.Mean().Val1)
	avgG := int(img.Mean().Val2)
	avgR := int(img.Mean().Val3)
	// log.Printf("averages")
	// log.Printf("blue: %v", avgB)
	// log.Printf("green: %v", avgG)
	// log.Printf("red: %v", avgR)

	//calculate median for each color
	medB, medG, medR := -1, -1, -1
	// medians := [3]*int{&medB, &medG, &medR}
	// for j, m := range medians {
	// 	bin := 0
	// 	channel := s[j]
	// 	for i := 0; i < 256 && *m < 0.0; i++ {
	// 		bin += int(channel.GetFloatAt(i, 0))
	// 		if bin > (totalPixels/2) && *m < 0.0 {
	// 			*m = i
	// 		}
	// 	}
	// }
	// // log.Printf("medians")
	// log.Printf("blue: %v", medB)
	// log.Printf("green: %v", medG)
	// log.Printf("red: %v", medR)
	return [3]int{avgB, avgG, avgR}, [3]int{medB, medG, medR}
}

func extractValuesHSV(img gocv.Mat) ([3]int, [3]int) {
	hsvMat := gocv.NewMat()
	defer hsvMat.Close()
	//convert img to HSV
	gocv.CvtColor(img, &hsvMat, gocv.ColorBGRToHSV)

	avgH := int(hsvMat.Mean().Val1)
	avgS := int(hsvMat.Mean().Val2)
	avgV := int(hsvMat.Mean().Val3)
	// log.Printf("averages")
	// log.Printf("Hue: %v", avgH)
	// log.Printf("Saturation: %v", avgS)
	// log.Printf("Value: %v", avgV)

	//calculate median for each color
	medH, medS, medV := -1, -1, -1
	// medians := [3]*int{&medH, &medS, &medV}
	// for j, m := range medians {
	// 	bin := 0
	// 	channel := split[j]
	// 	if j == 1 {
	// 		for i := 0; i < 180 && *m < 0.0; i++ {
	// 			bin += int(channel.GetFloatAt(i, 0))
	// 			if bin > (totalPixels/2) && *m < 0.0 {
	// 				*m = i
	// 			}
	// 		}
	// 	} else {
	// 		for i := 0; i < 256 && *m < 0.0; i++ {
	// 			bin += int(channel.GetFloatAt(i, 0))
	// 			if bin > (totalPixels/2) && *m < 0.0 {
	// 				*m = i
	// 			}
	// 		}
	// 	}
	// }
	// log.Printf("medians")
	// log.Printf("Hue: %v", medH)
	// log.Printf("Saturation: %v", medS)
	// log.Printf("Value: %v", medV)

	return [3]int{avgH, avgS, avgV}, [3]int{medH, medS, medV}
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
