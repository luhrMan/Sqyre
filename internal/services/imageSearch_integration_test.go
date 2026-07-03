package services

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/testdb"
	"Sqyre/internal/vision"
	"image"
	"image/color"
	"image/png"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func TestImageSearchFrameCloseOnZeroMats(t *testing.T) {
	var f imageSearchFrame
	f.leftX = 10
	f.topY = 20
	// Must not panic or segfault when mats were never allocated.
	f.Close()
}

func TestZIntegrationMatchImageSearchFrameSynthetic(t *testing.T) {
	repositories.ResetAllForTesting()
	testdb.SetupYAMLConfig(t)
	ResetSearchCacheForTesting()
	iconVariantServiceInstance = nil

	tempIconsDir := t.TempDir()
	prev := iconVariantServiceInstance
	iconVariantServiceInstance = &IconVariantService{basePath: tempIconsDir}
	t.Cleanup(func() {
		iconVariantServiceInstance = prev
		ResetSearchCacheForTesting()
	})

	programName := "search-test-" + strings.ReplaceAll(t.Name(), "/", "-")
	itemName := "test-item"
	program := models.NewProgram()
	program.Name = programName
	if err := repositories.ProgramRepo().Set(program.GetKey(), program); err != nil {
		t.Fatalf("save program: %v", err)
	}

	programIconsDir := filepath.Join(tempIconsDir, programName)
	if err := os.MkdirAll(programIconsDir, 0755); err != nil {
		t.Fatalf("mkdir icons: %v", err)
	}
	iconPath := filepath.Join(programIconsDir, itemName+config.PNG)
	if err := writeSolidPNG(iconPath, 24, 24, color.RGBA{R: 200, G: 100, B: 50, A: 255}); err != nil {
		t.Fatalf("write icon: %v", err)
	}

	item := &models.Item{Name: itemName, GridSize: [2]int{1, 1}}
	itemRepo, err := program.ItemRepo()
	if err != nil {
		t.Fatalf("item repo: %v", err)
	}
	if err := itemRepo.Set(itemName, item); err != nil {
		t.Fatalf("save item: %v", err)
	}
	InvalidateSearchTemplateCache(programName, itemName)

	targetKey := programName + config.ProgramDelimiter + itemName
	job := buildTargetMatchJob(targetKey)
	if job.program == nil || job.item == nil {
		t.Fatalf("buildTargetMatchJob missing program/item for %s", targetKey)
	}
	if len(job.variants) == 0 {
		variants, vErr := IconVariantServiceInstance().GetVariants(programName, itemName)
		t.Fatalf("buildTargetMatchJob variants empty for %s (GetVariants err=%v variants=%v icon=%s)", targetKey, vErr, variants, iconPath)
	}

	vision.WithOpenCV(func() {
		template, err := getCachedBlurredTemplate(iconPath, searchBlurKernel(5))
		if err != nil {
			t.Fatalf("load template: %v", err)
		}
		defer vision.CloseMat(&template)
		if template.Empty() {
			t.Fatal("template mat is empty")
		}
	})

	search := gocv.NewMatWithSize(120, 120, gocv.MatTypeCV8UC3)
	defer search.Close()
	search.SetTo(gocv.NewScalar(128, 128, 128, 0))
	patch := search.Region(image.Rect(40, 40, 64, 64))
	patch.SetTo(gocv.NewScalar(50, 100, 200, 0)) // BGR for 200,100,50 RGB icon
	patch.Close()

	frame := &imageSearchFrame{
		img:   search.Clone(),
		leftX: 0,
		topY:  0,
	}
	defer frame.Close()
	frame.imgDraw = frame.img.Clone()
	vision.WithOpenCV(func() {
		frame.searchImg = frame.img.Clone()
	})

	action := actions.NewImageSearch("find", nil, []string{targetKey}, "", 1, 1, 0.1, 0)
	results, err := matchImageSearchFrame(frame, action, nil)
	if err != nil {
		t.Fatalf("matchImageSearchFrame: %v", err)
	}
	pts := results[targetKey]
	if len(pts) == 0 {
		t.Fatalf("expected match for %s, got none (icon=%s variants=%v)", targetKey, iconPath, job.variants)
	}
	if !matchNear(pts, 52, 52, 12) {
		t.Fatalf("expected match near template center (52,52), got %v", pts)
	}
}

func matchNear(matches []robotgo.Point, x, y, tolerance int) bool {
	for _, m := range matches {
		dx, dy := m.X-x, m.Y-y
		if dx < 0 {
			dx = -dx
		}
		if dy < 0 {
			dy = -dy
		}
		if dx <= tolerance && dy <= tolerance {
			return true
		}
	}
	return false
}

func writeSolidPNG(path string, w, h int, c color.Color) error {
	img := image.NewRGBA(image.Rect(0, 0, w, h))
	for y := range h {
		for x := range w {
			img.Set(x, y, c)
		}
	}
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return png.Encode(f, img)
}
