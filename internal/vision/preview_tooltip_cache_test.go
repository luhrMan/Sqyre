package vision

import (
	"image"
	"testing"
	"time"

	"Sqyre/internal/models"
)

func TestPreviewTooltipCache_hitMissAndTTL(t *testing.T) {
	t.Helper()
	ResetPreviewTooltipCacheForTesting()
	t.Cleanup(ResetPreviewTooltipCacheForTesting)

	pt := &models.Point{Name: "origin", X: 10, Y: 20}
	key := previewCacheKeyPoint(pt)
	putPreviewTooltipCached(key, image.NewRGBA(image.Rect(0, 0, 2, 2)), "cap")

	if _, _, ok := getPreviewTooltipCached(key); !ok {
		t.Fatal("expected cache hit")
	}

	InvalidatePreviewTooltipCacheEntity("origin")
	if _, _, ok := getPreviewTooltipCached(key); ok {
		t.Fatal("expected cache miss after entity invalidation")
	}

	putPreviewTooltipCached(key, image.NewRGBA(image.Rect(0, 0, 2, 2)), "cap")
	previewTooltipCache.mu.Lock()
	previewTooltipCache.entries[key] = previewTooltipCacheEntry{
		image:   previewTooltipCache.entries[key].image,
		caption: "cap",
		expires: time.Now().Add(-time.Second),
	}
	previewTooltipCache.mu.Unlock()
	if _, _, ok := getPreviewTooltipCached(key); ok {
		t.Fatal("expected expired entry to miss")
	}
}

func TestPreviewTooltipCache_LRUEviction(t *testing.T) {
	t.Helper()
	ResetPreviewTooltipCacheForTesting()
	t.Cleanup(ResetPreviewTooltipCacheForTesting)

	img := image.NewRGBA(image.Rect(0, 0, 1, 1))
	for i := range previewTooltipCacheMaxEntries + 1 {
		key := previewCacheKeyPoint(&models.Point{Name: "p", X: i, Y: i})
		putPreviewTooltipCached(key, img, "c")
	}

	previewTooltipCache.mu.Lock()
	count := len(previewTooltipCache.entries)
	previewTooltipCache.mu.Unlock()
	if count != previewTooltipCacheMaxEntries {
		t.Fatalf("cache size = %d, want %d", count, previewTooltipCacheMaxEntries)
	}

	firstKey := previewCacheKeyPoint(&models.Point{Name: "p", X: 0, Y: 0})
	if _, _, ok := getPreviewTooltipCached(firstKey); ok {
		t.Fatal("expected oldest entry to be evicted")
	}
}
