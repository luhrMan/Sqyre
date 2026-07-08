package services

import (
	"Sqyre/internal/capture"
	"Sqyre/internal/config"
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/panicsafe"
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
	"sync/atomic"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

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

// imageSearchFrame holds captured and preprocessed mats for one screen snapshot.
type imageSearchFrame struct {
	img       gocv.Mat
	imgDraw   gocv.Mat
	searchImg gocv.Mat
	leftX     int
	topY      int
}

func (f *imageSearchFrame) Close() {
	if f == nil {
		return
	}
	vision.CloseMat(&f.img)
	vision.CloseMat(&f.imgDraw)
	vision.CloseMat(&f.searchImg)
}

func imageSearch(a *actions.ImageSearch, macro *models.Macro) (results map[string][]robotgo.Point, originX, originY int, err error) {
	defer func() {
		if r := recover(); r != nil {
			panicsafe.LogPanicToFile(r, "Image Search")
			if results == nil {
				results = make(map[string][]robotgo.Point)
			}
			originX, originY = 0, 0
			err = fmt.Errorf("image search panic: %v", r)
			log.Printf("Image Search: %v (macro continues)", err)
		}
	}()
	frame, leftX, topY, err := captureImageSearchFrame(a, macro)
	if err != nil {
		return nil, leftX, topY, err
	}
	defer frame.Close()
	results, err = matchImageSearchFrame(frame, a, macro)
	return results, frame.leftX, frame.topY, err
}

func captureImageSearchFrame(a *actions.ImageSearch, macro *models.Macro) (*imageSearchFrame, int, int, error) {
	leftX, topY, rightX, bottomY, err := macropkg.ResolveSearchAreaCoordsFromRef(a.SearchArea, macro, macropkg.DefaultResolutionKey())
	if err != nil {
		log.Printf("Image search: failed to resolve search area %q: %v", a.SearchArea, err)
		return nil, 0, 0, err
	}
	captureImg, leftX, topY, rightX, bottomY, err := capture.CaptureSearchArea(leftX, topY, rightX, bottomY)
	if err != nil {
		log.Printf("Image Search: %v (macro continues)", err)
		return nil, leftX, topY, err
	}
	w := rightX - leftX
	h := bottomY - topY
	log.Printf("Image Searching | %v in X1:%d Y1:%d X2:%d Y2:%d, width:%d height:%d", a.Targets, leftX, topY, rightX, bottomY, w, h)

	var frame imageSearchFrame
	frame.leftX = leftX
	frame.topY = topY
	var matErr error
	vision.WithOpenCV(func() {
		frame.img, matErr = gocv.ImageToMatRGB(captureImg)
		if matErr != nil {
			return
		}
		if frame.img.Empty() {
			matErr = fmt.Errorf("screen capture produced empty image (%dx%d area)", w, h)
			vision.CloseMat(&frame.img)
			return
		}
		SaveMetaImageLocked("searcharea", frame.img)
		frame.imgDraw = frame.img.Clone()
		frame.searchImg = blurForSearch(frame.img, a.Blur)
	})
	if matErr != nil {
		log.Println("image search failed:", matErr)
		frame.Close()
		return nil, leftX, topY, matErr
	}
	return &frame, leftX, topY, nil
}

type targetMatchJob struct {
	targetKey   string
	programName string
	itemName    string
	item        *models.Item
	program     *models.Program
	variants    []string
}

func prepareImageSearchJobs(targets []string) []targetMatchJob {
	jobs := make([]targetMatchJob, len(targets))
	for i, t := range targets {
		jobs[i] = buildTargetMatchJob(t)
	}
	return jobs
}

func buildTargetMatchJob(t string) targetMatchJob {
	job := targetMatchJob{targetKey: t}
	parts := strings.SplitN(t, config.ProgramDelimiter, 2)
	if len(parts) != 2 {
		log.Printf("Image Search: invalid target %q (expected program%vitem)", t, config.ProgramDelimiter)
		return job
	}
	job.programName, job.itemName = parts[0], parts[1]
	program, err := repositories.ProgramRepo().Get(job.programName)
	if err != nil {
		log.Printf("Error getting program %s: %v", job.programName, err)
		return job
	}
	job.program = program
	itemRepo, err := program.ItemRepo()
	if err != nil {
		log.Printf("Error getting item repo for program %s: %v", job.programName, err)
		return job
	}
	item, err := itemRepo.Get(job.itemName)
	if err != nil {
		log.Printf("Error getting item %s from program %s: %v", job.itemName, job.programName, err)
		return job
	}
	job.item = item
	variants, err := getCachedVariants(job.programName, job.itemName)
	if err != nil {
		log.Println("could not find variants for item during image search")
		return job
	}
	job.variants = variants
	return job
}

func matchImageSearchFrame(frame *imageSearchFrame, a *actions.ImageSearch, macro *models.Macro) (results map[string][]robotgo.Point, err error) {
	defer func() {
		if r := recover(); r != nil {
			panicsafe.LogPanicToFile(r, "Image Search match")
			if results == nil {
				results = make(map[string][]robotgo.Point)
			}
			err = fmt.Errorf("image search match panic: %v", r)
			log.Printf("Image Search: %v (macro continues)", err)
		}
	}()
	if frame == nil || frame.searchImg.Empty() {
		return make(map[string][]robotgo.Point), nil
	}

	jobs := prepareImageSearchJobs(a.Targets)
	results = make(map[string][]robotgo.Point, len(jobs))
	blurKernel := searchBlurKernel(a.Blur)
	vs := IconVariantServiceInstance()

	vision.WithOpenCV(func() {
		for _, job := range jobs {
			if job.item == nil || job.program == nil {
				results[job.targetKey] = nil
				continue
			}
			allMatches := []robotgo.Point{}
			for _, v := range job.variants {
				iconPath := vs.GetVariantPath(job.programName, job.itemName, v)
				template, tmplErr := getCachedBlurredTemplate(iconPath, blurKernel)
				if tmplErr != nil {
					log.Printf("Could not load icon %s: %v", iconPath, tmplErr)
					continue
				}
				func() {
					defer vision.CloseMat(&template)
					cmask := buildMaskLocked(job.item, job.program, template.Rows(), template.Cols(), macro)
					defer vision.CloseMat(&cmask)
					SaveMetaImageLocked("cmask-"+job.item.Name+"-"+v, cmask)

					matches := findTemplateMatches(frame.searchImg, template, cmask, a.Tolerance, a.Blur, true)
					DrawFoundMatches(matches, template.Cols(), template.Rows(), frame.imgDraw, job.item.Name)
					halfW := template.Cols() / 2
					halfH := template.Rows() / 2
					for j := range matches {
						matches[j].X += halfW
						matches[j].Y += halfH
					}
					allMatches = append(allMatches, matches...)
				}()
			}
			results[job.targetKey] = allMatches
		}
		SaveMetaImageLocked("founditems", frame.imgDraw)
	})
	return results, nil
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

// buildMaskLocked builds a match mask. Must be called while holding the OpenCV lock.
func buildMaskLocked(item *models.Item, program *models.Program, templateRows, templateCols int, macro *models.Macro) gocv.Mat {
	if item.Mask == "" {
		return gocv.NewMat()
	}

	maskRepo, err := program.MaskRepo()
	if err != nil {
		log.Printf("mask repo for program %s: %v", program.Name, err)
		return gocv.NewMat()
	}
	mask, err := maskRepo.Get(item.Mask)
	if err != nil {
		log.Printf("mask %q not found for item %s: %v", item.Mask, item.Name, err)
		return gocv.NewMat()
	}

	imgPath := filepath.Join(config.GetMasksPath(), program.Name, mask.Name+config.PNG)
	if _, statErr := os.Stat(imgPath); statErr == nil {
		if cached, ok := getCachedImageMask(imgPath, templateRows, templateCols); ok {
			return cached
		}
		log.Printf("mask image %s could not be loaded", imgPath)
		return gocv.NewMat()
	}

	maskVars := maskItemVariableOverrides(item, templateCols, templateRows)

	centerXPct, err := macropkg.ResolveIntWithOverrides(mask.CenterX, macro, maskVars)
	if err != nil {
		centerXPct = 50
	}
	centerYPct, err := macropkg.ResolveIntWithOverrides(mask.CenterY, macro, maskVars)
	if err != nil {
		centerYPct = 50
	}
	cx := clamp(templateCols*centerXPct/100, 0, templateCols-1)
	cy := clamp(templateRows*centerYPct/100, 0, templateRows-1)
	bounds := image.Rect(0, 0, templateCols, templateRows)

	var m gocv.Mat
	switch mask.Shape {
	case "circle":
		radius, err := macropkg.ResolveIntWithOverrides(mask.Radius, macro, maskVars)
		if err != nil || radius <= 0 {
			log.Printf("mask %q: invalid radius %v: %v", mask.Name, mask.Radius, err)
			return gocv.NewMat()
		}
		m = gocv.NewMatWithSize(templateRows, templateCols, gocv.MatTypeCV8U)
		m.SetTo(gocv.NewScalar(255, 255, 255, 0))
		gocv.Circle(&m, image.Point{X: cx, Y: cy}, radius, color.RGBA{0, 0, 0, 0}, -1)

	case "rectangle":
		base, err := macropkg.ResolveIntWithOverrides(mask.Base, macro, maskVars)
		if err != nil || base <= 0 {
			log.Printf("mask %q: invalid base %v: %v", mask.Name, mask.Base, err)
			return gocv.NewMat()
		}
		height, err := macropkg.ResolveIntWithOverrides(mask.Height, macro, maskVars)
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
func searchBlurKernel(blur int) int {
	if blur <= 0 {
		blur = 5
	}
	if blur%2 == 0 {
		blur++
	}
	return blur
}

// blurForSearch returns a blurred clone of the search image. Must run under the OpenCV lock.
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
		matches = findTemplateMatches(searchImg, template, cmask, threshold, blur, false)
	})
	return matches
}

func findTemplateMatches(searchImg, template, cmask gocv.Mat, threshold float32, blur int, templatePreBlurred bool) (matches []robotgo.Point) {
	defer func() {
		if r := recover(); r != nil {
			panicsafe.LogPanicToFile(r, "FindTemplateMatches")
		}
	}()
	if err := validateMatchInputs(searchImg, template, cmask, blur); err != nil {
		log.Printf("Image Search: skipping template match: %v", err)
		return nil
	}
	result := gocv.NewMat()
	defer vision.CloseMat(&result)

	t := template
	var owned gocv.Mat
	if !templatePreBlurred {
		owned = template.Clone()
		t = owned
		defer vision.CloseMat(&owned)
		kernel := image.Point{X: searchBlurKernel(blur), Y: searchBlurKernel(blur)}
		gocv.GaussianBlur(t, &t, kernel, 0, 0, gocv.BorderDefault)
	}

	gocv.MatchTemplate(searchImg, t, &result, gocv.TemplateMatchMode(5), cmask)
	return GetMatchesFromTemplateMatchResult(result, threshold, ImageSearchCloseMatchesDistance())
}

type matchPointDedup struct {
	distance int
	buckets  map[[2]int][]robotgo.Point
}

func newMatchPointDedup(distance int) *matchPointDedup {
	if distance < 0 {
		distance = 0
	}
	return &matchPointDedup{
		distance: distance,
		buckets:  make(map[[2]int][]robotgo.Point),
	}
}

func (d *matchPointDedup) addIfFar(point robotgo.Point) bool {
	if d.distance <= 0 {
		d.buckets[[2]int{0, 0}] = append(d.buckets[[2]int{0, 0}], point)
		return true
	}
	cell := d.distance + 1
	bx := point.X / cell
	by := point.Y / cell
	for dy := -1; dy <= 1; dy++ {
		for dx := -1; dx <= 1; dx++ {
			key := [2]int{bx + dx, by + dy}
			for _, existing := range d.buckets[key] {
				if abs(existing.X-point.X) <= d.distance && abs(existing.Y-point.Y) <= d.distance {
					return false
				}
			}
		}
	}
	key := [2]int{bx, by}
	d.buckets[key] = append(d.buckets[key], point)
	return true
}

func GetMatchesFromTemplateMatchResult(result gocv.Mat, threshold float32, closeMatchesDistance int) []robotgo.Point {
	resultRows := result.Rows()
	resultCols := result.Cols()
	if resultRows <= 0 || resultCols <= 0 {
		return nil
	}

	dedup := newMatchPointDedup(closeMatchesDistance)

	data, err := result.DataPtrFloat32()
	if err != nil || len(data) < resultRows*resultCols {
		return getMatchesFromTemplateMatchResultSlow(result, threshold, dedup, resultRows, resultCols)
	}

	var matches []robotgo.Point
	for y := range resultRows {
		row := y * resultCols
		for x := range resultCols {
			confidence := data[row+x]
			if confidence < threshold || math.IsInf(float64(confidence), 0) || math.IsNaN(float64(confidence)) {
				continue
			}
			newPoint := robotgo.Point{X: x, Y: y}
			if dedup.addIfFar(newPoint) {
				matches = append(matches, newPoint)
			}
		}
	}
	return matches
}

func getMatchesFromTemplateMatchResultSlow(result gocv.Mat, threshold float32, dedup *matchPointDedup, resultRows, resultCols int) []robotgo.Point {
	var matches []robotgo.Point
	for y := range resultRows {
		for x := range resultCols {
			confidence := result.GetFloatAt(y, x)
			if confidence < threshold || math.IsInf(float64(confidence), 0) || math.IsNaN(float64(confidence)) {
				continue
			}
			newPoint := robotgo.Point{X: x, Y: y}
			if dedup.addIfFar(newPoint) {
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
			panicsafe.LogPanicToFile(r, "DrawFoundMatches")
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
type NamedPoint struct {
	Name  string
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

func CalculateCircleMask(rows, cols int, center image.Point, radius int) func() *gocv.Mat {
	return func() *gocv.Mat {
		cmask := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV8U)
		cmask.SetTo(gocv.NewScalar(0, 0, 0, 0))
		gocv.Circle(&cmask, center, radius, color.RGBA{255, 255, 255, 0}, -1)
		return &cmask
	}
}
