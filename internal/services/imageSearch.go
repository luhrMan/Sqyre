package services

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"fmt"
	"image"
	"image/color"
	"log"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func imageSearch(a *actions.ImageSearch, macro *models.Macro) (map[string][]robotgo.Point, error) {
	sa := a.SearchArea
	leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
	if err != nil {
		log.Printf("Image search: failed to resolve search area coords: %v", err)
		return nil, err
	}
	w := rightX - leftX
	h := bottomY - topY
	if w <= 0 || h <= 0 {
		err := fmt.Errorf("image search: invalid search area (width=%d height=%d); need positive dimensions", w, h)
		log.Printf("Image Search: %v (macro continues)", err)
		return nil, err
	}
	log.Printf("Image Searching | %v in X1:%d Y1:%d X2:%d Y2:%d, width:%d height:%d", a.Targets, leftX, topY, rightX, bottomY, w, h)
	captureImg, err := robotgo.CaptureImg(leftX, topY, w, h)
	if err != nil {
		log.Printf("Image Search: capture failed: %v (macro continues)", err)
		return nil, err
	}
	if captureImg == nil {
		err := fmt.Errorf("image search: capture returned nil (invalid search area or display)")
		log.Printf("Image Search: %v (macro continues)", err)
		return nil, err
	}
	img, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		log.Println("image search failed:", err)
		return nil, err
	}
	defer img.Close()
	SaveMetaImage("searcharea", img)

	imgDraw := img.Clone()
	defer imgDraw.Close()

	results, err := match(img, imgDraw, a, macro)
	if err != nil {
		log.Printf("Image Search: %v", err)
		return results, err
	}
	return results, nil
}

func match(img, imgDraw gocv.Mat, a *actions.ImageSearch, macro *models.Macro) (map[string][]robotgo.Point, error) {
	vs := IconVariantServiceInstance()
	Imask := gocv.NewMat()
	defer Imask.Close()

	results := make(map[string][]robotgo.Point)
	var wg sync.WaitGroup
	resultsMutex := &sync.Mutex{}
	var matchErr error
	for _, t := range a.Targets {
		wg.Add(1)
		go func(t string) {
			defer wg.Done()
			defer func() {
				if r := recover(); r != nil {
					LogPanicToFile(r)
					resultsMutex.Lock()
					matchErr = fmt.Errorf("panic during image search for %s: %v", t, r)
					resultsMutex.Unlock()
					log.Printf("Image Search: recovered from panic for target %s: %v", t, r)
				}
			}()
			programName := strings.Split(t, config.ProgramDelimiter)[0]
			program, err := repositories.ProgramRepo().Get(programName)
			if err != nil {
				log.Printf("Error getting program %s: %v", programName, err)
				return
			}
			itemName := strings.Split(t, config.ProgramDelimiter)[1]
			i, err := program.ItemRepo().Get(itemName)
			if err != nil {
				log.Printf("Error getting item %s from program %s: %v", itemName, programName, err)
				return
			}
			variants, err := vs.GetVariants(programName, itemName)
			if err != nil {
				log.Println("could not find variants for item during image search")
			}

			// Accumulate all matches for this item across all variants
			allMatches := []robotgo.Point{}

			for _, v := range variants { // search for the item variants also
				ip := programName + config.ProgramDelimiter + itemName + config.ProgramDelimiter + v + config.PNG

				// Load icon on-demand from cache
				resource := assets.GetFyneResource(ip)
				if resource == nil {
					log.Printf("Could not load icon resource for %s", ip)
					continue
				}
				b := resource.Content()

				template := gocv.NewMat()
				defer template.Close()
				err = gocv.IMDecodeIntoMat(b, gocv.IMReadColor, &template)

				if err != nil {
					log.Printf("Error reading template image: %v", err)
					return
				}
			tmask := gocv.NewMat()
			cmask := buildMask(i, program, template.Rows(), template.Cols(), macro)

			defer tmask.Close()
			defer cmask.Close()
			SaveMetaImage("cmask-"+i.Name+"-"+v, cmask)

				// if Tmask.Cols() != template.Cols() && Tmask.Rows() != template.Rows() {
				// 	log.Println("ERROR: template mask size does not match template!")
				// 	log.Println("item: ", t)
				// 	log.Println("Tmask cols: ", Tmask.Cols())
				// 	log.Println("Tmask rows: ", Tmask.Rows())
				// 	log.Println("t cols: ", template.Cols())
				// 	log.Println("t rows: ", template.Rows())
				// 	return
				// }

				matches := FindTemplateMatches(img, template, Imask, tmask, cmask, a.Tolerance, a.Blur)
				DrawFoundMatches(matches, template.Cols(), template.Rows(), imgDraw, i.Name) // draw rect at top-left
				// Offset each match to the center of the icon
				halfW := template.Cols() / 2
				halfH := template.Rows() / 2
				for i := range matches {
					matches[i].X += halfW
					matches[i].Y += halfH
				}
				allMatches = append(allMatches, matches...)
			}

			// Store accumulated matches once per item
			resultsMutex.Lock()
			results[t] = allMatches
			resultsMutex.Unlock()
		}(t)
	}
	wg.Wait()
	SaveMetaImage("founditems", imgDraw)
	return results, matchErr
}

func buildMask(item *models.Item, program *models.Program, templateRows, templateCols int, macro *models.Macro) gocv.Mat {
	if item.Mask == "" {
		return gocv.NewMat()
	}

	mask, err := program.MaskRepo().Get(item.Mask)
	if err != nil {
		log.Printf("mask %q not found for item %s: %v", item.Mask, item.Name, err)
		return gocv.NewMat()
	}

	// Image-based mask takes precedence over shape-based
	imgPath := filepath.Join(config.GetMasksPath(), program.Name, mask.Name+config.PNG)
	if _, statErr := os.Stat(imgPath); statErr == nil {
		m := gocv.IMRead(imgPath, gocv.IMReadGrayScale)
		if m.Empty() {
			log.Printf("mask image %s could not be loaded", imgPath)
			return gocv.NewMat()
		}
		if m.Rows() != templateRows || m.Cols() != templateCols {
			gocv.Resize(m, &m, image.Point{X: templateCols, Y: templateRows}, 0, 0, gocv.InterpolationLinear)
		}
		gocv.Threshold(m, &m, 127, 255, gocv.ThresholdBinary)
		return m
	}

	centerXPct, err := ResolveInt(mask.CenterX, macro)
	if err != nil {
		centerXPct = 50
	}
	centerYPct, err := ResolveInt(mask.CenterY, macro)
	if err != nil {
		centerYPct = 50
	}
	cx := clamp(templateCols*centerXPct/100, 0, templateCols-1)
	cy := clamp(templateRows*centerYPct/100, 0, templateRows-1)
	bounds := image.Rect(0, 0, templateCols, templateRows)

	switch mask.Shape {
	case "circle":
		radius, err := ResolveInt(mask.Radius, macro)
		if err != nil || radius <= 0 {
			log.Printf("mask %q: invalid radius %v: %v", mask.Name, mask.Radius, err)
			return gocv.NewMat()
		}
		m := gocv.NewMatWithSize(templateRows, templateCols, gocv.MatTypeCV8U)
		m.SetTo(gocv.NewScalar(0, 0, 0, 0))
		gocv.Circle(&m, image.Point{X: cx, Y: cy}, radius, color.RGBA{255, 255, 255, 0}, -1)
		return m

	case "rectangle":
		base, err := ResolveInt(mask.Base, macro)
		if err != nil || base <= 0 {
			log.Printf("mask %q: invalid base %v: %v", mask.Name, mask.Base, err)
			return gocv.NewMat()
		}
		height, err := ResolveInt(mask.Height, macro)
		if err != nil || height <= 0 {
			log.Printf("mask %q: invalid height %v: %v", mask.Name, mask.Height, err)
			return gocv.NewMat()
		}
		rect := image.Rect(cx-base/2, cy-height/2, cx+base/2, cy+height/2).Intersect(bounds)
		if rect.Empty() {
			log.Printf("mask %q: rectangle fully outside template (%dx%d)", mask.Name, templateCols, templateRows)
			return gocv.NewMat()
		}
		m := gocv.NewMatWithSize(templateRows, templateCols, gocv.MatTypeCV8U)
		m.SetTo(gocv.NewScalar(255, 255, 255, 0))
		region := m.Region(rect)
		region.SetTo(gocv.NewScalar(0, 0, 0, 0))
		region.Close()
		return m

	default:
		log.Printf("mask %q: unknown shape %q", mask.Name, mask.Shape)
		return gocv.NewMat()
	}
}

func FindTemplateMatches(img, template, imask, tmask, cmask gocv.Mat, threshold float32, blur int) []robotgo.Point {
	result := gocv.NewMat()
	defer result.Close()

	i := img.Clone()
	t := template.Clone()
	defer i.Close()
	defer t.Close()
	if blur <= 0 {
		blur = 5
	}
	kernel := image.Point{X: blur, Y: blur}

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
	gocv.MatchTemplate(i, t, &result, gocv.TemplateMatchMode(5), cmask)
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
				log.Printf("Position (%d, %d), Correlation: %.4f",
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

func clamp(v, lo, hi int) int {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}
	return v
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
	// gocv.IMWrite("./internal/resources/images/meta/imagetext-test.png", i)
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

// NamedPoint pairs a match point with its target key (programName~itemName).
// Used so Image Search sub-actions can resolve item internal variables (StackMax, Cols, Rows, etc.).
type NamedPoint struct {
	Name  string // target key: programName + ProgramDelimiter + itemName
	Point robotgo.Point
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

func SortListOfPoints(lop map[string][]robotgo.Point) []NamedPoint {
	var list []NamedPoint
	for name, points := range lop {
		for _, pt := range points {
			list = append(list, NamedPoint{Name: name, Point: pt})
		}
	}
	// Same order as SortPoints(..., "TopLeftToBottomRight")
	sort.Slice(list, func(i, j int) bool {
		pi, pj := list[i].Point, list[j].Point
		for a := 0; a <= 5; a++ {
			if pi.Y+a == pj.Y || pi.Y == pj.Y+a {
				return pi.X < pj.X
			}
		}
		return pi.Y < pj.Y
	})
	return list
}

// func CalculateCornerMask(rows, cols int, r image.Rectangle) *gocv.Mat {
// 	cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
// 	cmask.SetTo(gocv.NewScalar(255, 255, 255, 0))

// 	region := cmask.Region(r) // = gocv.NewScalar(0, 0, 0, 0) //.SetTo(gocv.NewScalar(0, 0, 0, 0))
// 	defer region.Close()
// 	region.SetTo(gocv.NewScalar(0, 0, 0, 0))

// 	return &cmask
// }

func CalculateCornerMask(rows, cols int, r image.Rectangle) func() *gocv.Mat {
	return func() *gocv.Mat {
		cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
		cmask.SetTo(gocv.NewScalar(255, 255, 255, 0))

		region := cmask.Region(r)
		defer region.Close()
		region.SetTo(gocv.NewScalar(0, 0, 0, 0))

		return &cmask
	}
}

// CalculateCircleMask creates a mask with a filled circle within the image.
func CalculateCircleMask(rows, cols int, center image.Point, radius int) func() *gocv.Mat {
	return func() *gocv.Mat {
		cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
		// Fill with zeros (fully masked)
		cmask.SetTo(gocv.NewScalar(0, 0, 0, 0))
		// Then draw a filled white circle (unmasked)
		gocv.Circle(&cmask, center, radius, color.RGBA{255, 255, 255, 0}, -1)
		return &cmask
	}
}
