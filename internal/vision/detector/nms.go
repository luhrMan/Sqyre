package detector

import (
	"image"
	"sort"
)

type scoredBox struct {
	label      string
	confidence float32
	box        image.Rectangle
}

// nonMaxSuppression filters overlapping boxes, keeping highest-confidence first.
func nonMaxSuppression(boxes []scoredBox, iouThreshold float32, maxMatches int) []Detection {
	if len(boxes) == 0 {
		return nil
	}
	sort.Slice(boxes, func(i, j int) bool {
		return boxes[i].confidence > boxes[j].confidence
	})

	kept := make([]Detection, 0, len(boxes))
	suppressed := make([]bool, len(boxes))

	for i, cand := range boxes {
		if suppressed[i] {
			continue
		}
		kept = append(kept, Detection{
			Label:      cand.label,
			Confidence: cand.confidence,
			Bounds:     cand.box,
		})
		if maxMatches > 0 && len(kept) >= maxMatches {
			break
		}
		for j := i + 1; j < len(boxes); j++ {
			if suppressed[j] || boxes[j].label != cand.label {
				continue
			}
			if iou(boxes[j].box, cand.box) >= iouThreshold {
				suppressed[j] = true
			}
		}
	}
	return kept
}

func iou(a, b image.Rectangle) float32 {
	inter := a.Intersect(b)
	if inter.Empty() {
		return 0
	}
	interArea := float32(inter.Dx() * inter.Dy())
	union := float32(a.Dx()*a.Dy()+b.Dx()*b.Dy()) - interArea
	if union <= 0 {
		return 0
	}
	return interArea / union
}
