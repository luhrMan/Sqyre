package services

import (
	"Sqyre/internal/config"
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/vision"
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
	"sync/atomic"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

const maxImageSearchWorkers = 4

var imageSearchCloseMatchesDistance atomic.Int32

func init() {
	imageSearchCloseMatchesDistance.Store(config.DefaultImageSearchCloseMatchesDistance)
}

// SetImageSearchCloseMatchesDistance sets how many pixels apart duplicate template
// matches are treated as the same find. Updated from user settings at startup and
// when the preference changes.
func SetImageSearchCloseMatchesDistance(v int) {
	if v < 0 {
		v = 0
	}
	imageSearchCloseMatchesDistance.Store(int32(v))
}

// ImageSearchCloseMatchesDistance returns the current close-match distance in pixels.
func ImageSearchCloseMatchesDistance() int {
	return int(imageSearchCloseMatchesDistance.Load())
}

func imageSearch(a *actions.ImageSearch, macro *models.Macro) (results map[string][]robotgo.Point, originX, originY int, err error) {
	defer func() {
		if r := recover(); r != nil {
			macropkg.LogPanicToFile(r, "Image Search")
			if results == nil {
				results = make(map[string][]robotgo.Point)
			}
			originX, originY = 0, 0
			err = fmt.Errorf("image search panic: %v", r)
			log.Printf("Image Search: %v (macro continues)", err)
		}
	}()
	return imageSearchCapture(a, macro)
}

func imageSearchCapture(a *actions.ImageSearch, macro *models.Macro) (map[string][]robotgo.Point, int, int, error) {
	leftX, topY, rightX, bottomY, err := macropkg.ResolveSearchAreaCoordsFromRef(a.SearchArea, macro, macropkg.DefaultResolutionKey())
	if err != nil {
		log.Printf("Image search: failed to resolve search area %q: %v", a.SearchArea, err)
		return nil, 0, 0, err
	}
	captureImg, leftX, topY, rightX, bottomY, err := macropkg.CaptureSearchArea(leftX, topY, rightX, bottomY)
	if err != nil {
		log.Printf("Image Search: %v (macro continues)", err)
		return nil, leftX, topY, err
	}
	w := rightX - leftX
	h := bottomY - topY
	log.Printf("Image Searching | %v in X1:%d Y1:%d X2:%d Y2:%d, width:%d height:%d", a.Targets, leftX, topY, rightX, bottomY, w, h)
	var img gocv.Mat
	var matErr error
	vision.WithOpenCV(func() {
		img, matErr = gocv.ImageToMatRGB(captureImg)
	})
	if matErr != nil {
		log.Println("image search failed:", matErr)
		return nil, leftX, topY, matErr
	}
	defer img.Close()
	if img.Empty() {
		err := fmt.Errorf("screen capture produced empty image (%dx%d area)", w, h)
		log.Printf("Image Search: %v (macro continues)", err)
		return nil, leftX, topY, err
	}
	SaveMetaImage("searcharea", img)

	var imgDraw gocv.Mat
	vision.WithOpenCV(func() { imgDraw = img.Clone() })
	defer imgDraw.Close()

	results, err := match(img, imgDraw, a, macro)
	if err != nil {
		log.Printf("Image Search: %v (macro continues)", err)
		return results, leftX, topY, err
	}
	return results, leftX, topY, nil
}

func match(img, imgDraw gocv.Mat, a *actions.ImageSearch, macro *models.Macro) (results map[string][]robotgo.Point, err error) {
	defer func() {
		if r := recover(); r != nil {
			macropkg.LogPanicToFile(r, "Image Search match")
			if results == nil {
				results = make(map[string][]robotgo.Point)
			}
			err = fmt.Errorf("image search match panic: %v", r)
			log.Printf("Image Search: %v (macro continues)", err)
		}
	}()
	return matchParallel(img, imgDraw, a, macro)
}

func matchParallel(img, imgDraw gocv.Mat, a *actions.ImageSearch, macro *models.Macro) (map[string][]robotgo.Point, error) {
	vs := IconVariantServiceInstance()

	var searchImg gocv.Mat
	vision.WithOpenCV(func() { searchImg = blurForSearch(img, a.Blur) })
	defer searchImg.Close()

	results := make(map[string][]robotgo.Point)
	var wg sync.WaitGroup
	resultsMutex := &sync.Mutex{}
	drawMutex := &sync.Mutex{}
	workers := maxImageSearchWorkers
	if workers > len(a.Targets) {
		workers = len(a.Targets)
	}
	if workers < 1 {
		workers = 1
	}
	sem := make(chan struct{}, workers)
	var matchErr error

	for _, t := range a.Targets {
		wg.Add(1)
		go func(t string) {
			sem <- struct{}{}
			defer func() { <-sem }()
			defer wg.Done()
			defer func() {
				if r := recover(); r != nil {
					macropkg.LogPanicToFile(r, fmt.Sprintf("Image Search (target %s)", t))
					resultsMutex.Lock()
					matchErr = fmt.Errorf("panic during image search for %s: %v", t, r)
					resultsMutex.Unlock()
				}
			}()

			var localSearch gocv.Mat
			vision.WithOpenCV(func() { localSearch = searchImg.Clone() })
			defer localSearch.Close()
			if localSearch.Empty() {
				log.Printf("Image Search: empty search clone for target %s", t)
				return
			}

			parts := strings.SplitN(t, config.ProgramDelimiter, 2)
			if len(parts) != 2 {
				log.Printf("Image Search: invalid target %q (expected program%vitem)", t, config.ProgramDelimiter)
				return
			}
			programName, itemName := parts[0], parts[1]
			program, err := repositories.ProgramRepo().Get(programName)
			if err != nil {
				log.Printf("Error getting program %s: %v", programName, err)
				return
			}
			i, err := program.ItemRepo().Get(itemName)
			if err != nil {
				log.Printf("Error getting item %s from program %s: %v", itemName, programName, err)
				return
			}
			variants, err := vs.GetVariants(programName, itemName)
			if err != nil {
				log.Println("could not find variants for item during image search")
			}

			allMatches := []robotgo.Point{}
			for _, v := range variants {
				iconPath := vs.GetVariantPath(programName, itemName, v)
				iconBytes, readErr := os.ReadFile(iconPath)
				if readErr != nil {
					log.Printf("Could not load icon %s: %v", iconPath, readErr)
					continue
				}

				func() {
					defer func() {
						if r := recover(); r != nil {
							macropkg.LogPanicToFile(r, fmt.Sprintf("Image Search (target %s variant %s)", t, v))
							resultsMutex.Lock()
							matchErr = fmt.Errorf("panic during image search for %s: %v", t, r)
							resultsMutex.Unlock()
						}
					}()

					template := gocv.NewMat()
					defer template.Close()
					if err := gocv.IMDecodeIntoMat(iconBytes, gocv.IMReadColor, &template); err != nil {
						log.Printf("Error reading template image: %v", err)
						return
					}
					cmask := buildMask(i, program, template.Rows(), template.Cols(), macro)
					defer cmask.Close()
					SaveMetaImage("cmask-"+i.Name+"-"+v, cmask)

					matches := findTemplateMatches(localSearch, template, cmask, a.Tolerance, a.Blur)
					drawMutex.Lock()
					DrawFoundMatches(matches, template.Cols(), template.Rows(), imgDraw, i.Name)
					drawMutex.Unlock()
					halfW := template.Cols() / 2
					halfH := template.Rows() / 2
					for j := range matches {
						matches[j].X += halfW
						matches[j].Y += halfH
					}
					allMatches = append(allMatches, matches...)
				}()
			}

			resultsMutex.Lock()
			results[t] = allMatches
			resultsMutex.Unlock()
		}(t)
	}
	wg.Wait()
	SaveMetaImage("founditems", imgDraw)
	return results, matchErr
}

func maskItemVariableOverrides(item *models.Item, templateCols, templateRows int) map[string]any {
	overrides := map[string]any{
		"ImagePixelWidth":  templateCols,
		"ImagePixelHeight": templateRows,
	}
	if item != nil {
		overrides["ItemName"] = item.Name
		overrides["StackMax"] = item.StackMax
		overrides["Cols"] = item.GridSize[0]
		overrides["Rows"] = item.GridSize[1]
	}
	return overrides
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
			resized := gocv.NewMat()
			gocv.Resize(m, &resized, image.Point{X: templateCols, Y: templateRows}, 0, 0, gocv.InterpolationLinear)
			m.Close()
			m = resized
		}
		gocv.Threshold(m, &m, 127, 255, gocv.ThresholdBinary)
		return m
	}

	maskVars := maskItemVariableOverrides(item, templateCols, templateRows)

	centerXPct, err := ResolveIntWithOverrides(mask.CenterX, macro, maskVars)
	if err != nil {
		centerXPct = 50
	}
	centerYPct, err := ResolveIntWithOverrides(mask.CenterY, macro, maskVars)
	if err != nil {
		centerYPct = 50
	}
	cx := clamp(templateCols*centerXPct/100, 0, templateCols-1)
	cy := clamp(templateRows*centerYPct/100, 0, templateRows-1)
	bounds := image.Rect(0, 0, templateCols, templateRows)

	var m gocv.Mat
	switch mask.Shape {
	case "circle":
		radius, err := ResolveIntWithOverrides(mask.Radius, macro, maskVars)
		if err != nil || radius <= 0 {
			log.Printf("mask %q: invalid radius %v: %v", mask.Name, mask.Radius, err)
			return gocv.NewMat()
		}
		m = gocv.NewMatWithSize(templateRows, templateCols, gocv.MatTypeCV8U)
		m.SetTo(gocv.NewScalar(255, 255, 255, 0))
		gocv.Circle(&m, image.Point{X: cx, Y: cy}, radius, color.RGBA{0, 0, 0, 0}, -1)

	case "rectangle":
		base, err := ResolveIntWithOverrides(mask.Base, macro, maskVars)
		if err != nil || base <= 0 {
			log.Printf("mask %q: invalid base %v: %v", mask.Name, mask.Base, err)
			return gocv.NewMat()
		}
		height, err := ResolveIntWithOverrides(mask.Height, macro, maskVars)
		if err != nil || height <= 0 {
			log.Printf("mask %q: invalid height %v: %v", mask.Name, mask.Height, err)
			return gocv.NewMat()
		}
		rect := image.Rect(cx-base/2, cy-height/2, cx+base/2, cy+height/2).Intersect(bounds)
		if rect.Empty() {
			log.Printf("mask %q: rectangle fully outside template (%dx%d)", mask.Name, templateCols, templateRows)
			return gocv.NewMat()
		}
		m = gocv.NewMatWithSize(templateRows, templateCols, gocv.MatTypeCV8U)
		m.SetTo(gocv.NewScalar(255, 255, 255, 0))
		region := m.Region(rect)
		region.SetTo(gocv.NewScalar(0, 0, 0, 0))
		region.Close()

	default:
		log.Printf("mask %q: unknown shape %q", mask.Name, mask.Shape)
		return gocv.NewMat()
	}

	if mask.Inverse {
		gocv.BitwiseNot(m, &m)
	}
	return m
}

// validateMatchInputs guards against OpenCV preconditions that otherwise cause CGo segfaults.
func validateMatchInputs(img, template, cmask gocv.Mat, blur int) error {
	if img.Empty() || template.Empty() {
		return fmt.Errorf("empty search image or template")
	}
	imgRows, imgCols := img.Rows(), img.Cols()
	tmplRows, tmplCols := template.Rows(), template.Cols()
	if imgRows <= 0 || imgCols <= 0 || tmplRows <= 0 || tmplCols <= 0 {
		return fmt.Errorf("invalid image/template dimensions (%dx%d vs %dx%d)", imgCols, imgRows, tmplCols, tmplRows)
	}
	if tmplRows > imgRows || tmplCols > imgCols {
		return fmt.Errorf("template (%dx%d) larger than search image (%dx%d)", tmplCols, tmplRows, imgCols, imgRows)
	}
	if !cmask.Empty() && (cmask.Rows() != tmplRows || cmask.Cols() != tmplCols) {
		return fmt.Errorf("mask (%dx%d) does not match template (%dx%d)", cmask.Cols(), cmask.Rows(), tmplCols, tmplRows)
	}
	if !cmask.Empty() && cmask.Type() != gocv.MatTypeCV8U {
		return fmt.Errorf("mask must be 8-bit grayscale (got type %d)", cmask.Type())
	}
	if blur <= 0 {
		blur = 5
	}
	if blur%2 == 0 {
		blur++
	}
	if blur > imgRows || blur > imgCols || blur > tmplRows || blur > tmplCols {
		return fmt.Errorf("blur kernel %d too large for image (%dx%d) or template (%dx%d)", blur, imgCols, imgRows, tmplCols, tmplRows)
	}
	return nil
}

// searchBlurKernel normalizes a blur amount to a positive odd Gaussian kernel size.
// GaussianBlur requires an odd, positive kernel; default to 5 when unset.
func searchBlurKernel(blur int) int {
	if blur <= 0 {
		blur = 5
	}
	if blur%2 == 0 {
		blur++
	}
	return blur
}

// blurForSearch returns a blurred clone of the search image using the shared
// kernel. If the kernel is too large for the image, the unblurred clone is
// returned (validateMatchInputs will skip the match in that case anyway).
func blurForSearch(img gocv.Mat, blur int) gocv.Mat {
	out := img.Clone()
	k := searchBlurKernel(blur)
	if k <= img.Rows() && k <= img.Cols() {
		gocv.GaussianBlur(out, &out, image.Point{X: k, Y: k}, 0, 0, gocv.BorderDefault)
	}
	return out
}

// FindTemplateMatches matches template against searchImg, which must already be
// blurred via blurForSearch using the same blur amount.
func FindTemplateMatches(searchImg, template, imask, tmask, cmask gocv.Mat, threshold float32, blur int) (matches []robotgo.Point) {
	_ = imask
	_ = tmask
	vision.WithOpenCV(func() {
		matches = findTemplateMatches(searchImg, template, cmask, threshold, blur)
	})
	return matches
}

func findTemplateMatches(searchImg, template, cmask gocv.Mat, threshold float32, blur int) (matches []robotgo.Point) {
	defer func() {
		if r := recover(); r != nil {
			macropkg.LogPanicToFile(r, "FindTemplateMatches")
		}
	}()
	if err := validateMatchInputs(searchImg, template, cmask, blur); err != nil {
		log.Printf("Image Search: skipping template match: %v", err)
		return nil
	}
	result := gocv.NewMat()
	defer result.Close()

	t := template.Clone()
	defer t.Close()
	kernel := image.Point{X: searchBlurKernel(blur), Y: searchBlurKernel(blur)}
	gocv.GaussianBlur(t, &t, kernel, 0, 0, gocv.BorderDefault)

	gocv.MatchTemplate(searchImg, t, &result, gocv.TemplateMatchMode(5), cmask)
	return GetMatchesFromTemplateMatchResult(result, threshold, ImageSearchCloseMatchesDistance())
}

func GetMatchesFromTemplateMatchResult(result gocv.Mat, threshold float32, closeMatchesDistance int) []robotgo.Point {
	resultRows := result.Rows()
	resultCols := result.Cols()
	if resultRows <= 0 || resultCols <= 0 {
		return nil
	}

	// Read the result buffer once instead of a CGo GetFloatAt call per cell.
	data, err := result.DataPtrFloat32()
	if err != nil || len(data) < resultRows*resultCols {
		return getMatchesFromTemplateMatchResultSlow(result, threshold, closeMatchesDistance, resultRows, resultCols)
	}

	var matches []robotgo.Point
	for y := 0; y < resultRows; y++ {
		row := y * resultCols
		for x := 0; x < resultCols; x++ {
			confidence := data[row+x]
			if confidence < threshold || math.IsInf(float64(confidence), 0) || math.IsNaN(float64(confidence)) {
				continue
			}
			newPoint := robotgo.Point{X: x, Y: y}
			if !isNearExistingPoint(newPoint, matches, closeMatchesDistance) {
				matches = append(matches, newPoint)
			}
		}
	}

	return matches
}

// getMatchesFromTemplateMatchResultSlow is the fallback path used when the
// result buffer cannot be accessed directly (non-contiguous Mat).
func getMatchesFromTemplateMatchResultSlow(result gocv.Mat, threshold float32, closeMatchesDistance, resultRows, resultCols int) []robotgo.Point {
	var matches []robotgo.Point
	for y := 0; y < resultRows; y++ {
		for x := 0; x < resultCols; x++ {
			confidence := result.GetFloatAt(y, x)
			if confidence < threshold || math.IsInf(float64(confidence), 0) || math.IsNaN(float64(confidence)) {
				continue
			}
			newPoint := robotgo.Point{X: x, Y: y}
			if !isNearExistingPoint(newPoint, matches, closeMatchesDistance) {
				matches = append(matches, newPoint)
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

func DrawFoundMatches(matches []robotgo.Point, rectSizeX, rectSizeY int, draw gocv.Mat, text string) {
	defer func() {
		if r := recover(); r != nil {
			macropkg.LogPanicToFile(r, "DrawFoundMatches")
		}
	}()
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
