package structs

import (
	"Dark-And-Darker/utils"
	"bytes"
	"fmt"
	"image"
	"image/png"
	"log"
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
	robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/temp.jpeg")
	// capture := robotgo.CaptureScreen(a.SearchBox.LeftX, a.SearchBox.TopY, w, h)
	defer robotgo.FreeBitmap(capture)

	var wg sync.WaitGroup
	results := make(map[string][]robotgo.Point)

	resultsMutex := &sync.Mutex{}

	for _, target := range a.Targets {
		wg.Add(1)
		go func(target string) {
			defer wg.Done()

			ip := "./images/icons/" + target + ".png"
			// Read the main image and template
		    img := gocv.IMRead("./images/temp.jpeg", gocv.IMReadColor)
		    if img.Empty() {
		        fmt.Println("Error reading main image")
		        return
		    }
		    defer img.Close()

		    template := gocv.IMRead(ip, gocv.IMReadColor)
		    if template.Empty() {
		        fmt.Println("Error reading template image")
		        return
		    }
		    defer template.Close()

		    // Print image information for debugging
		    fmt.Printf("Main image: Type=%v, Channels=%v",
		        img.Type(), img.Channels())
		    fmt.Printf("Template: Type=%v, Channels=%v",
		        template.Type(), template.Channels())

			resultMat := gocv.NewMat()
			resultMat.Close()
			maskMat := gocv.NewMat()
			maskMat.Close()
//			targetResults, err := locateimage.All(context.Background(), captureConvert, predefinedImage, 0.15)
//			if err != nil {
//				log.Print(err)
//			}
			//gocv.MatchTemplate(captureImg, imgMat, &resultMat, 5, maskMat)
			matches := findTemplateMatches(img, template, 0.8)
			resultsMutex.Lock()
			results[target] = matches
			resultsMutex.Unlock()

			log.Printf("Results for %s: %v\n", target, matches)


		}(target)
	}

	wg.Wait()

	for _, pointArr := range results {
		for _, point := range pointArr {
			point.X += a.SearchBox.LeftX
			point.Y += a.SearchBox.TopY
			for _, d := range a.SubActions {
				d.Execute(point)
			}
		}
	}
	log.Printf("Total # found: %v\n", len(results))
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

type MatchResult struct {
    X, Y int
}
func findTemplateMatches(img gocv.Mat, template gocv.Mat, threshold float32) []robotgo.Point {
	// Convert images to grayscale
    gray := gocv.NewMat()
    defer gray.Close()
    gocv.CvtColor(img, &gray, gocv.ColorBGRToGray)

    templateGray := gocv.NewMat()
    defer templateGray.Close()
    gocv.CvtColor(template, &templateGray, gocv.ColorBGRToGray)

    // Ensure both images are 8-bit
    gray8bit := gocv.NewMat()
    defer gray8bit.Close()
    templateGray8bit := gocv.NewMat()
    defer templateGray8bit.Close()

	gray.ConvertTo(&gray8bit, gocv.MatTypeCV8U)
    templateGray.ConvertTo(&templateGray8bit, gocv.MatTypeCV8U)

	// Create the result matrix
    result := gocv.NewMat()
    defer result.Close()

    // Perform template matching
	mask := gocv.NewMat()
	defer mask.Close()
    gocv.MatchTemplate(img, template, &result, gocv.TmCcoeffNormed, mask)
    // Get the dimensions
    resultRows := result.Rows()
    resultCols := result.Cols()

    // Store matches
    var matches []robotgo.Point

    // Iterate through the result matrix
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