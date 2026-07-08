package macro

import (
	"fmt"
	"testing"

	"fyne.io/fyne/v2/canvas"
)

func TestRowContentLRUCache_evictsOldest(t *testing.T) {
	t.Helper()
	cache := newRowContentLRUCache()
	entry := cachedRowContent{display: canvas.NewRectangle(nil)}
	for i := range rowContentCacheMaxEntries + 1 {
		cache.put(fmt.Sprintf("uid%d", i), entry)
	}
	if len(cache.data) != rowContentCacheMaxEntries {
		t.Fatalf("cache size = %d, want %d", len(cache.data), rowContentCacheMaxEntries)
	}
	if _, ok := cache.get("uid0"); ok {
		t.Fatal("expected oldest uid to be evicted")
	}
	if _, ok := cache.get(fmt.Sprintf("uid%d", rowContentCacheMaxEntries)); !ok {
		t.Fatal("expected newest uid to remain")
	}
}

func TestRowContentLRUCache_touchOnGet(t *testing.T) {
	t.Helper()
	cache := newRowContentLRUCache()
	entry := cachedRowContent{display: canvas.NewRectangle(nil)}
	cache.put("a", entry)
	cache.put("b", entry)
	if _, ok := cache.get("a"); !ok {
		t.Fatal("expected hit for a")
	}
	if cache.order[len(cache.order)-1] != "a" {
		t.Fatal("expected get to mark entry as most recently used")
	}
}
