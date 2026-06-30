package services

import (
	"Sqyre/internal/config"
	"Sqyre/internal/vision"
	"image"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	"gocv.io/x/gocv"
)

type searchCache struct {
	mu sync.RWMutex

	variants   map[string]variantListCacheEntry
	templates  map[string]templateCacheEntry
	imageMasks map[string]imageMaskCacheEntry
}

type variantListCacheEntry struct {
	variants []string
	dirMTime time.Time
}

type templateCacheEntry struct {
	blurred    gocv.Mat
	modTime    time.Time
	blurKernel int
}

type imageMaskCacheEntry struct {
	mask    gocv.Mat
	modTime time.Time
}

var globalSearchCache = &searchCache{
	variants:   make(map[string]variantListCacheEntry),
	templates:  make(map[string]templateCacheEntry),
	imageMasks: make(map[string]imageMaskCacheEntry),
}

func templateCacheKey(iconPath string, blurKernel int) string {
	return iconPath + "\x00" + strconv.Itoa(blurKernel)
}

func variantCacheKey(iconsPath, programName, itemName string) string {
	return iconsPath + "\x00" + programName + config.ProgramDelimiter + itemName
}

func imageMaskCacheKey(path string, rows, cols int) string {
	return path + "\x00" + strconv.Itoa(rows) + "\x00" + strconv.Itoa(cols)
}

func iconsDirMTime(iconsPath string) time.Time {
	info, err := os.Stat(iconsPath)
	if err != nil {
		return time.Time{}
	}
	return info.ModTime()
}

func closeCachedMats(mats []gocv.Mat) {
	if len(mats) == 0 {
		return
	}
	vision.WithOpenCV(func() {
		for i := range mats {
			vision.CloseMat(&mats[i])
		}
	})
}

// InvalidateSearchTemplateCache drops cached templates and variant lists for one item.
func InvalidateSearchTemplateCache(programName, itemName string) {
	var toClose []gocv.Mat

	globalSearchCache.mu.Lock()
	iconsPath := IconVariantServiceInstance().getIconsPath(programName)
	delete(globalSearchCache.variants, variantCacheKey(iconsPath, programName, itemName))

	prefix := filepath.Join(iconsPath, itemName)
	for key, entry := range globalSearchCache.templates {
		if strings.HasPrefix(key, prefix) {
			toClose = append(toClose, entry.blurred)
			delete(globalSearchCache.templates, key)
		}
	}
	globalSearchCache.mu.Unlock()

	closeCachedMats(toClose)
}

// InvalidateSearchTemplateCacheProgram drops all search caches for a program.
func InvalidateSearchTemplateCacheProgram(programName string) {
	var toClose []gocv.Mat

	globalSearchCache.mu.Lock()

	iconsPath := IconVariantServiceInstance().getIconsPath(programName)
	variantPrefix := iconsPath + "\x00" + programName + config.ProgramDelimiter
	for key := range globalSearchCache.variants {
		if strings.HasPrefix(key, variantPrefix) {
			delete(globalSearchCache.variants, key)
		}
	}

	for key, entry := range globalSearchCache.templates {
		if strings.HasPrefix(key, iconsPath) {
			toClose = append(toClose, entry.blurred)
			delete(globalSearchCache.templates, key)
		}
	}

	maskPrefix := filepath.Join(config.GetMasksPath(), programName)
	for key, entry := range globalSearchCache.imageMasks {
		if strings.HasPrefix(key, maskPrefix) {
			toClose = append(toClose, entry.mask)
			delete(globalSearchCache.imageMasks, key)
		}
	}
	globalSearchCache.mu.Unlock()

	closeCachedMats(toClose)
}

func getCachedVariants(programName, itemName string) ([]string, error) {
	vs := IconVariantServiceInstance()
	iconsPath := IconVariantServiceInstance().getIconsPath(programName)
	dirMTime := iconsDirMTime(iconsPath)
	key := variantCacheKey(iconsPath, programName, itemName)

	globalSearchCache.mu.RLock()
	if entry, ok := globalSearchCache.variants[key]; ok && entry.dirMTime.Equal(dirMTime) {
		out := append([]string(nil), entry.variants...)
		globalSearchCache.mu.RUnlock()
		return out, nil
	}
	globalSearchCache.mu.RUnlock()

	variants, err := vs.GetVariants(programName, itemName)
	if err != nil {
		return nil, err
	}

	globalSearchCache.mu.Lock()
	globalSearchCache.variants[key] = variantListCacheEntry{
		variants: append([]string(nil), variants...),
		dirMTime: dirMTime,
	}
	globalSearchCache.mu.Unlock()
	return variants, nil
}

// getCachedBlurredTemplate returns a clone of the blurred template for iconPath.
// Must be called while holding the OpenCV lock.
func getCachedBlurredTemplate(iconPath string, blurKernel int) (gocv.Mat, error) {
	info, err := os.Stat(iconPath)
	if err != nil {
		return gocv.Mat{}, err
	}
	modTime := info.ModTime()
	key := templateCacheKey(iconPath, blurKernel)

	globalSearchCache.mu.RLock()
	entry, ok := globalSearchCache.templates[key]
	cacheHit := ok && entry.modTime.Equal(modTime) && entry.blurKernel == blurKernel && entry.blurred.Ptr() != nil && !entry.blurred.Empty()
	if cacheHit {
		cloned := entry.blurred.Clone()
		globalSearchCache.mu.RUnlock()
		return cloned, nil
	}
	globalSearchCache.mu.RUnlock()

	iconBytes, err := os.ReadFile(iconPath)
	if err != nil {
		return gocv.Mat{}, err
	}
	template := gocv.NewMat()
	if err := gocv.IMDecodeIntoMat(iconBytes, gocv.IMReadColor, &template); err != nil {
		vision.CloseMat(&template)
		return gocv.Mat{}, err
	}
	blurred := blurTemplateForSearch(template, blurKernel)
	vision.CloseMat(&template)

	globalSearchCache.mu.Lock()
	var oldMat gocv.Mat
	if old, ok := globalSearchCache.templates[key]; ok {
		oldMat = old.blurred
	}
	globalSearchCache.templates[key] = templateCacheEntry{
		blurred:    blurred.Clone(),
		modTime:    modTime,
		blurKernel: blurKernel,
	}
	cloned := blurred.Clone()
	globalSearchCache.mu.Unlock()
	vision.CloseMat(&blurred)
	vision.CloseMat(&oldMat)
	return cloned, nil
}

// getCachedImageMask returns a clone of a file-based mask resized to template dimensions.
// Must be called while holding the OpenCV lock.
func getCachedImageMask(imgPath string, templateRows, templateCols int) (gocv.Mat, bool) {
	info, err := os.Stat(imgPath)
	if err != nil {
		return gocv.Mat{}, false
	}
	key := imageMaskCacheKey(imgPath, templateRows, templateCols)

	globalSearchCache.mu.RLock()
	entry, ok := globalSearchCache.imageMasks[key]
	cacheHit := ok && entry.modTime.Equal(info.ModTime()) && entry.mask.Ptr() != nil && !entry.mask.Empty()
	if cacheHit {
		cloned := entry.mask.Clone()
		globalSearchCache.mu.RUnlock()
		return cloned, true
	}
	globalSearchCache.mu.RUnlock()

	m := gocv.IMRead(imgPath, gocv.IMReadGrayScale)
	if m.Empty() {
		vision.CloseMat(&m)
		return gocv.Mat{}, false
	}
	if m.Rows() != templateRows || m.Cols() != templateCols {
		resized := gocv.NewMat()
		gocv.Resize(m, &resized, image.Point{X: templateCols, Y: templateRows}, 0, 0, gocv.InterpolationLinear)
		vision.CloseMat(&m)
		m = resized
	}
	gocv.Threshold(m, &m, 127, 255, gocv.ThresholdBinary)

	globalSearchCache.mu.Lock()
	var oldMat gocv.Mat
	if old, ok := globalSearchCache.imageMasks[key]; ok {
		oldMat = old.mask
	}
	globalSearchCache.imageMasks[key] = imageMaskCacheEntry{
		mask:    m.Clone(),
		modTime: info.ModTime(),
	}
	cloned := m.Clone()
	globalSearchCache.mu.Unlock()
	vision.CloseMat(&m)
	vision.CloseMat(&oldMat)
	return cloned, true
}

func blurTemplateForSearch(template gocv.Mat, blur int) gocv.Mat {
	out := template.Clone()
	k := searchBlurKernel(blur)
	if k <= out.Rows() && k <= out.Cols() {
		gocv.GaussianBlur(out, &out, image.Point{X: k, Y: k}, 0, 0, gocv.BorderDefault)
	}
	return out
}
