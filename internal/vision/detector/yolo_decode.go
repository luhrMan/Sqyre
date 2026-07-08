package detector

import "math"

// decodeYOLOWorldOutput parses YOLOv8-World ONNX output0 shaped [4+numClasses, numAnchors].
// Box values are center-x, center-y, width, height in letterbox pixel space.
func decodeYOLOWorldOutput(
	output []float32,
	numClasses int,
	numAnchors int,
	labels []string,
	lb letterboxResult,
	confidenceThreshold float32,
) []scoredBox {
	if numClasses <= 0 || numAnchors <= 0 || len(labels) == 0 {
		return nil
	}
	rowStride := numAnchors
	expected := (4 + numClasses) * numAnchors
	if len(output) < expected {
		return nil
	}

	var boxes []scoredBox
	for a := 0; a < numAnchors; a++ {
		cx := output[0*rowStride+a]
		cy := output[1*rowStride+a]
		w := output[2*rowStride+a]
		h := output[3*rowStride+a]

		bestClass := -1
		var bestScore float32
		for c := 0; c < numClasses; c++ {
			logit := output[(4+c)*rowStride+a]
			score := sigmoid(logit)
			if score > bestScore {
				bestScore = score
				bestClass = c
			}
		}
		if bestClass < 0 || bestScore < confidenceThreshold {
			continue
		}
		label := labels[bestClass]
		boxes = append(boxes, scoredBox{
			label:      label,
			confidence: bestScore,
			box:        lb.mapBox(cx, cy, w, h),
		})
	}
	return boxes
}

func sigmoid(x float32) float32 {
	return 1 / (1 + float32(math.Exp(float64(-x))))
}
