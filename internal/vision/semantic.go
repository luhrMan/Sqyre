package vision

import (
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/vision/detector"
	"context"
	"fmt"
	"image"
	"image/color"
	"log"

	"gocv.io/x/gocv"
)

// MacroUsesSemantic reports whether a macro tree contains a Semantic Search action.
func MacroUsesSemantic(m *models.Macro) bool {
	seen := make(map[string]bool)
	return macroUsesSemanticRec(m, seen)
}

func macroUsesSemanticRec(m *models.Macro, seen map[string]bool) bool {
	if m == nil || m.Root == nil {
		return false
	}
	if m.Name != "" {
		if seen[m.Name] {
			return false
		}
		seen[m.Name] = true
	}
	var uses bool
	models.WalkActions(m.Root, func(a actions.ActionInterface) {
		if uses {
			return
		}
		if _, ok := a.(*actions.SemanticSearch); ok {
			uses = true
			return
		}
		rm, ok := a.(*actions.RunMacro)
		if !ok || rm.MacroName == "" {
			return
		}
		target, err := repositories.MacroRepo().Get(rm.MacroName)
		if err == nil && macroUsesSemanticRec(target, seen) {
			uses = true
		}
	})
	return uses
}

// SemanticDetect captures the search area and runs open-vocabulary detection.
// Returns detections in search-area-local coordinates plus the search area origin.
func SemanticDetect(node *actions.SemanticSearch, macro *models.Macro, prompt string) (dets []detector.Detection, originX, originY int, err error) {
	prompts := detector.ParsePrompt(prompt)
	if len(prompts) == 0 {
		log.Printf("Semantic search: no prompts in %q", prompt)
		return nil, 0, 0, nil
	}

	leftX, topY, rightX, bottomY, err := macropkg.ResolveSearchAreaCoordsFromRef(node.SearchArea, macro, macropkg.DefaultResolutionKey())
	if err != nil {
		log.Printf("Semantic search: failed to resolve search area %q: %v", node.SearchArea, err)
		return nil, 0, 0, fmt.Errorf("resolve search area: %w", err)
	}
	captureImg, capLeftX, capTopY, _, _, err := macropkg.CaptureSearchArea(leftX, topY, rightX, bottomY)
	if err != nil {
		log.Printf("Semantic search: capture failed: %v", err)
		return nil, 0, 0, fmt.Errorf("capture search area: %w", err)
	}
	if captureImg == nil {
		log.Printf("Semantic search: empty capture")
		return nil, 0, 0, fmt.Errorf("empty capture")
	}
	saveSemanticSearchAreaMeta(captureImg)

	opts := detector.Options{
		Prompts:             prompts,
		ConfidenceThreshold: node.ConfidenceThreshold,
		IoUThreshold:        node.IoUThreshold,
		MaxMatches:          node.MaxMatches,
	}
	if opts.ConfidenceThreshold <= 0 {
		opts.ConfidenceThreshold = 0.25
	}
	if opts.IoUThreshold <= 0 {
		opts.IoUThreshold = 0.45
	}

	log.Printf("%q Semantic search | %v in %s at X1:%d Y1:%d X2:%d Y2:%d",
		prompt, prompts, node.SearchArea.DisplayLabel(), leftX, topY, rightX, bottomY)
	if !detector.Available() {
		log.Printf("Semantic search: vision worker unavailable (install sqyre-vision or set worker path in Settings)")
		return nil, 0, 0, fmt.Errorf("semantic vision unavailable: install sqyre-vision and run make vision-models")
	}
	dets, err = detector.GetDetector().Detect(context.Background(), captureImg, opts)
	if err != nil {
		log.Printf("Semantic search: detection failed: %v", err)
		return nil, capLeftX, capTopY, err
	}
	saveSemanticDetectionsMeta(captureImg, dets)
	log.Printf("Semantic search: %d match(es)", len(dets))
	return dets, capLeftX, capTopY, nil
}

func saveSemanticSearchAreaMeta(captureImg image.Image) {
	if captureImg == nil {
		return
	}
	WithOpenCV(func() {
		mat, err := gocv.ImageToMatRGB(captureImg)
		if err != nil || mat.Empty() {
			return
		}
		defer CloseMat(&mat)
		SaveMetaImageLocked("semantic-searcharea", mat)
	})
}

func saveSemanticDetectionsMeta(captureImg image.Image, dets []detector.Detection) {
	if captureImg == nil {
		return
	}
	WithOpenCV(func() {
		mat, err := gocv.ImageToMatRGB(captureImg)
		if err != nil || mat.Empty() {
			return
		}
		defer CloseMat(&mat)
		draw := mat.Clone()
		defer CloseMat(&draw)
		boxColor := color.RGBA{G: 220, A: 255}
		for _, d := range dets {
			DrawPreviewRectangle(&draw, d.Bounds, boxColor, 2)
			if d.Label == "" {
				continue
			}
			label := fmt.Sprintf("%s %.0f%%", d.Label, d.Confidence*100)
			pt := image.Pt(d.Bounds.Min.X, d.Bounds.Min.Y-2)
			if pt.Y < 12 {
				pt.Y = d.Bounds.Min.Y + 12
			}
			gocv.PutText(&draw, label, pt, gocv.FontHersheySimplex, 0.4, boxColor, 1)
		}
		SaveMetaImageLocked("semantic-detections", draw)
	})
}

// WarmUpDetector preloads the vision worker so the first semantic search is fast.
func WarmUpDetector() {
	if !detector.Available() {
		return
	}
	path := detector.ResolveWorkerPath()
	if path == "" {
		return
	}
	detector.WarmUpOnce(path)
}
