package vision

import (
	"Sqyre/internal/models"
	"fmt"
	"image"
	"image/draw"
	"strings"
	"sync"
	"time"
)

const (
	previewTooltipCacheMaxEntries = 24
	previewTooltipCacheTTL        = 30 * time.Second
)

type previewTooltipCacheEntry struct {
	image   image.Image
	caption string
	expires time.Time
}

var previewTooltipCache struct {
	mu      sync.Mutex
	order   []string
	entries map[string]previewTooltipCacheEntry
}

func previewCacheKeyPoint(pt *models.Point) string {
	if pt == nil {
		return ""
	}
	return fmt.Sprintf("pt:%s:%v:%v", pt.Name, pt.X, pt.Y)
}

func previewCacheKeySearchArea(sa *models.SearchArea) string {
	if sa == nil {
		return ""
	}
	return fmt.Sprintf("sa:%s:%v:%v:%v:%v", sa.Name, sa.LeftX, sa.TopY, sa.RightX, sa.BottomY)
}

func cloneImage(img image.Image) image.Image {
	if img == nil {
		return nil
	}
	b := img.Bounds()
	dst := image.NewRGBA(image.Rect(0, 0, b.Dx(), b.Dy()))
	draw.Draw(dst, dst.Bounds(), img, b.Min, draw.Src)
	return dst
}

func getPreviewTooltipCached(key string) (image.Image, string, bool) {
	if key == "" {
		return nil, "", false
	}
	now := time.Now()
	previewTooltipCache.mu.Lock()
	defer previewTooltipCache.mu.Unlock()
	entry, ok := previewTooltipCache.entries[key]
	if !ok || now.After(entry.expires) {
		if ok {
			delete(previewTooltipCache.entries, key)
			removePreviewCacheOrderKeyLocked(key)
		}
		return nil, "", false
	}
	touchPreviewCacheOrderLocked(key)
	return cloneImage(entry.image), entry.caption, true
}

func putPreviewTooltipCached(key string, img image.Image, caption string) {
	if key == "" || img == nil {
		return
	}
	previewTooltipCache.mu.Lock()
	defer previewTooltipCache.mu.Unlock()
	if previewTooltipCache.entries == nil {
		previewTooltipCache.entries = make(map[string]previewTooltipCacheEntry)
	}
	if _, ok := previewTooltipCache.entries[key]; !ok {
		previewTooltipCache.order = append(previewTooltipCache.order, key)
	}
	previewTooltipCache.entries[key] = previewTooltipCacheEntry{
		image:   cloneImage(img),
		caption: caption,
		expires: time.Now().Add(previewTooltipCacheTTL),
	}
	touchPreviewCacheOrderLocked(key)
	for len(previewTooltipCache.order) > previewTooltipCacheMaxEntries {
		evict := previewTooltipCache.order[0]
		previewTooltipCache.order = previewTooltipCache.order[1:]
		delete(previewTooltipCache.entries, evict)
	}
}

func touchPreviewCacheOrderLocked(key string) {
	for i, k := range previewTooltipCache.order {
		if k == key {
			previewTooltipCache.order = append(append(previewTooltipCache.order[:i:i], previewTooltipCache.order[i+1:]...), key)
			return
		}
	}
}

func removePreviewCacheOrderKeyLocked(key string) {
	for i, k := range previewTooltipCache.order {
		if k == key {
			previewTooltipCache.order = append(previewTooltipCache.order[:i], previewTooltipCache.order[i+1:]...)
			return
		}
	}
}

// InvalidatePreviewTooltipCache clears all cached hover preview images.
func InvalidatePreviewTooltipCache() {
	previewTooltipCache.mu.Lock()
	defer previewTooltipCache.mu.Unlock()
	previewTooltipCache.order = nil
	previewTooltipCache.entries = nil
}

// InvalidatePreviewTooltipCacheEntity drops cached previews for a point or search area name.
func InvalidatePreviewTooltipCacheEntity(entityName string) {
	if entityName == "" {
		InvalidatePreviewTooltipCache()
		return
	}
	prefixPt := "pt:" + entityName + ":"
	prefixSa := "sa:" + entityName + ":"
	previewTooltipCache.mu.Lock()
	defer previewTooltipCache.mu.Unlock()
	for key := range previewTooltipCache.entries {
		if strings.HasPrefix(key, prefixPt) || strings.HasPrefix(key, prefixSa) {
			delete(previewTooltipCache.entries, key)
			removePreviewCacheOrderKeyLocked(key)
		}
	}
}

// ResetPreviewTooltipCacheForTesting clears the hover preview cache (tests only).
func ResetPreviewTooltipCacheForTesting() {
	InvalidatePreviewTooltipCache()
}
