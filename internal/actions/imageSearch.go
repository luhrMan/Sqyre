package actions

import (
	"Squire/internal"
	"Squire/internal/structs"
	"Squire/internal/utils"
	"fmt"
	"image"
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
	captureImg := robotgo.CaptureImg(a.SearchBox.LeftX+utils.XOffset, a.SearchBox.TopY+utils.YOffset, w, h)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()
	pathDir := "internal/resources/images/"
	gocv.IMWrite(pathDir+"search-area.png", img)

	imgDraw := img.Clone()
	defer imgDraw.Close()

	results := a.match(pathDir, img, imgDraw)

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

func (a *ImageSearch) match(pathDir string, img, imgDraw gocv.Mat) map[string][]robotgo.Point {
	//	maskedIcons := *internal.MaskItems()
	icons := *internal.GetIconBytes()
	path := "./internal/resources/images/"
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
	case strings.Contains(a.SearchBox.Name, "Player Inventory Stash"):
		tolerance = 0.96
		Imask = gocv.IMRead(path+"empty-player-stash.png", gocv.IMReadColor)
	case strings.Contains(a.SearchBox.Name, "Stash"):
		tolerance = 0.96
		Imask = gocv.IMRead(path+"empty-stash.png", gocv.IMReadColor)
	case strings.Contains(a.SearchBox.Name, "Merchant"):
		tolerance = 0.93
		Imask = gocv.IMRead(path+"empty-player-merchant.png", gocv.IMReadColor)
	default:
		tolerance = 0.95
	}

	Tmask1x1 := gocv.IMRead(path+"masks/1x1 mask.png", gocv.IMReadColor)
	Tmask1x2 := gocv.IMRead(path+"masks/1x2 mask.png", gocv.IMReadColor)
	Tmask1x3 := gocv.IMRead(path+"masks/1x3 mask.png", gocv.IMReadColor)
	Tmask2x1 := gocv.IMRead(path+"masks/2x1 mask.png", gocv.IMReadColor)
	Tmask2x2 := gocv.IMRead(path+"masks/2x2 mask.png", gocv.IMReadColor)
	Tmask2x3 := gocv.IMRead(path+"masks/2x3 mask.png", gocv.IMReadColor)
	defer Tmask1x1.Close()
	defer Tmask1x2.Close()
	defer Tmask1x3.Close()
	defer Tmask2x1.Close()
	defer Tmask2x2.Close()
	defer Tmask2x3.Close()
	Cmask1x1 := gocv.IMRead(path+"masks/1x1 Cmask.png", gocv.IMReadGrayScale)
	Cmask1x2 := gocv.IMRead(path+"masks/1x2 Cmask.png", gocv.IMReadGrayScale)
	Cmask1x3 := gocv.IMRead(path+"masks/1x3 Cmask.png", gocv.IMReadGrayScale)
	Cmask2x1 := gocv.IMRead(path+"masks/2x1 Cmask.png", gocv.IMReadGrayScale)
	Cmask2x2 := gocv.IMRead(path+"masks/2x2 Cmask.png", gocv.IMReadGrayScale)
	Cmask2x3 := gocv.IMRead(path+"masks/2x3 Cmask.png", gocv.IMReadGrayScale)
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

			i, _ := internal.Items.GetItem(target)
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

			if Tmask.Cols() != template.Cols() && Tmask.Rows() != template.Rows() {
				log.Println("item: ", target)
				log.Println("Tmask cols: ", Tmask.Cols())
				log.Println("Tmask rows: ", Tmask.Rows())
				log.Println("t cols: ", template.Cols())
				log.Println("t rows: ", template.Rows())
				return
			}
			var matches []robotgo.Point
			matches = a.findTemplateMatches(img, template, Imask, Tmask, Cmask, tolerance)

			sort.Slice(matches, func(i, j int) bool {
				return matches[i].Y < matches[j].Y
			})

			resultsMutex.Lock()
			defer resultsMutex.Unlock()

			results[target] = matches
			utils.DrawFoundMatches(matches, xSize*i.GridSize[0], ySize*i.GridSize[1], imgDraw, target)
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

func (a *ImageSearch) findTemplateMatches(img, template, Imask, Tmask, Cmask gocv.Mat, threshold float32) []robotgo.Point {
	result := gocv.NewMat()
	defer result.Close()

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
	gocv.MatchTemplate(i, t, &result, gocv.TemplateMatchMode(5), Cmask)
	matches := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)
	return matches
}
