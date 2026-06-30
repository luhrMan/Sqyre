package services

import (
	"sync"
	"testing"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func TestMatchPointDedup(t *testing.T) {
	d := newMatchPointDedup(5)
	p1 := robotgo.Point{X: 10, Y: 10}
	p2 := robotgo.Point{X: 12, Y: 12}
	p3 := robotgo.Point{X: 20, Y: 20}
	if !d.addIfFar(p1) {
		t.Fatal("expected first point to be accepted")
	}
	if d.addIfFar(p2) {
		t.Fatal("expected nearby point to be rejected")
	}
	if !d.addIfFar(p3) {
		t.Fatal("expected distant point to be accepted")
	}
}

func TestGetMatchesFromTemplateMatchResult_findsPeak(t *testing.T) {
	result := gocv.NewMatWithSize(10, 10, gocv.MatTypeCV32FC1)
	defer result.Close()
	result.SetTo(gocv.NewScalar(0, 0, 0, 0))
	result.SetFloatAt(5, 7, 0.95)

	matches := GetMatchesFromTemplateMatchResult(result, 0.9, 10)
	if len(matches) != 1 || matches[0].X != 7 || matches[0].Y != 5 {
		t.Fatalf("got %v, want single match at (7,5)", matches)
	}
}

func TestFindTemplateMatchesConcurrentSharedSearchImage(t *testing.T) {
	base := gocv.NewMatWithSize(200, 200, gocv.MatTypeCV8UC3)
	defer base.Close()
	base.SetTo(gocv.NewScalar(128, 128, 128, 0))
	search := blurForSearch(base, 5)
	defer search.Close()

	template := gocv.NewMatWithSize(24, 24, gocv.MatTypeCV8UC3)
	defer template.Close()
	template.SetTo(gocv.NewScalar(200, 100, 50, 0))

	imask := gocv.NewMat()
	defer imask.Close()
	tmask := gocv.NewMat()
	defer tmask.Close()
	cmask := gocv.NewMat()
	defer cmask.Close()

	const workers = 16
	const iterations = 25

	var wg sync.WaitGroup
	wg.Add(workers)
	for w := 0; w < workers; w++ {
		go func() {
			defer wg.Done()
			for i := 0; i < iterations; i++ {
				_ = FindTemplateMatches(search, template, imask, tmask, cmask, 0.5, 5)
			}
		}()
	}
	wg.Wait()
}
