package editor

import (
	"Sqyre/internal/models"
	"testing"
)

func TestResolveSearchAreaBoundsRejectsNonPositiveDimensions(t *testing.T) {
	sa := &models.SearchArea{Name: "test"}
	b := searchAreaBounds{lx: 10, ty: 20, rx: 10, by: 50}
	_, err := resolveSearchAreaBounds("SearchArea", sa, b)
	if err == nil {
		t.Fatal("expected dimension error")
	}
}
