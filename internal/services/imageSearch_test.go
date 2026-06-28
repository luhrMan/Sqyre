package services

import (
	"sync"
	"testing"

	"gocv.io/x/gocv"
)

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
