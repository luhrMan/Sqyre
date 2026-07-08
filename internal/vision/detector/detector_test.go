package detector

import (
	"image"
	"testing"
)

func TestParsePrompt(t *testing.T) {
	tests := []struct {
		in   string
		want []string
	}{
		{"All Healing potions", []string{"healing potions"}},
		{"Metal Armor, Boots", []string{"metal armor", "boots"}},
		{"healing potion and mana potion", []string{"healing potion", "mana potion"}},
		{"  sword  ", []string{"sword"}},
		{"", nil},
	}
	for _, tc := range tests {
		got := ParsePrompt(tc.in)
		if len(got) != len(tc.want) {
			t.Fatalf("ParsePrompt(%q) = %v, want %v", tc.in, got, tc.want)
		}
		for i := range got {
			if got[i] != tc.want[i] {
				t.Fatalf("ParsePrompt(%q)[%d] = %q, want %q", tc.in, i, got[i], tc.want[i])
			}
		}
	}
}

func TestNonMaxSuppression(t *testing.T) {
	boxes := []scoredBox{
		{label: "a", confidence: 0.9, box: image.Rect(10, 10, 50, 50)},
		{label: "a", confidence: 0.85, box: image.Rect(12, 12, 52, 52)}, // overlaps first
		{label: "b", confidence: 0.8, box: image.Rect(200, 200, 240, 240)},
	}
	got := nonMaxSuppression(boxes, 0.5, 0)
	if len(got) != 2 {
		t.Fatalf("NMS kept %d boxes, want 2", len(got))
	}
	if got[0].Label != "a" || got[1].Label != "b" {
		t.Fatalf("unexpected labels: %+v", got)
	}
}

func TestDecodeYOLOWorldOutput(t *testing.T) {
	const numClasses = 1
	const numAnchors = 2
	out := make([]float32, (4+numClasses)*numAnchors)
	// Anchor 0: box at center of 640 letterbox, high class score
	out[0] = 320
	out[numAnchors] = 320
	out[2*numAnchors] = 40
	out[3*numAnchors] = 40
	out[4*numAnchors] = 5 // logit ~0.99 after sigmoid
	// Anchor 1: low score (should be filtered)
	out[4*numAnchors+1] = -10

	lb := letterboxResult{size: 640, scale: 1, srcWidth: 640, srcHeight: 640}
	boxes := decodeYOLOWorldOutput(out, numClasses, numAnchors, []string{"potion"}, lb, 0.25)
	if len(boxes) != 1 {
		t.Fatalf("got %d boxes, want 1", len(boxes))
	}
	if boxes[0].label != "potion" {
		t.Fatalf("label = %q", boxes[0].label)
	}
}

func TestStubDetector(t *testing.T) {
	d := StubDetector{}
	got, err := d.Detect(t.Context(), image.NewRGBA(image.Rect(0, 0, 10, 10)), Options{Prompts: []string{"x"}})
	if err != nil {
		t.Fatal(err)
	}
	if got != nil {
		t.Fatalf("stub should return nil detections, got %+v", got)
	}
}
