package serialize

import (
	"Sqyre/internal/models/actions"
	"testing"
)

func TestSemanticSearchRoundTrip(t *testing.T) {
	orig := actions.NewSemanticSearch(
		"find potions",
		[]actions.ActionInterface{actions.NewClick("left", false)},
		"All Healing potions",
		actions.NewCoordinateRef("Demo", "Inventory"),
	)
	orig.ConfidenceThreshold = 0.4
	orig.MaxMatches = 5
	orig.OutputLabelVariable = "foundLabel"
	orig.OutputXVariable = "x"
	orig.OutputYVariable = "y"

	m, err := encodeActionToMap(orig)
	if err != nil {
		t.Fatal(err)
	}
	decoded, err := decodeActionFromMap(m)
	if err != nil {
		t.Fatal(err)
	}
	ss, ok := decoded.(*actions.SemanticSearch)
	if !ok {
		t.Fatalf("got %T", decoded)
	}
	if ss.Prompt != orig.Prompt || ss.ConfidenceThreshold != orig.ConfidenceThreshold ||
		ss.MaxMatches != orig.MaxMatches || ss.OutputLabelVariable != orig.OutputLabelVariable {
		t.Fatalf("round-trip mismatch: %+v", ss)
	}
}
