package clipdata_test

import (
	"testing"

	"Sqyre/internal/vision/clipdata"
)

func TestEncodeTokenIDs_wrapsWithSpecialTokens(t *testing.T) {
	ids, mask, err := clipdata.EncodeTokenIDs("a photo of a cat")
	if err != nil {
		t.Fatal(err)
	}
	if len(ids) != clipdata.MaxTokens || len(mask) != clipdata.MaxTokens {
		t.Fatalf("length = %d / %d", len(ids), len(mask))
	}
	if ids[0] != clipdata.StartOfText {
		t.Fatalf("start token = %d", ids[0])
	}
	if mask[0] != 1 {
		t.Fatal("mask[0] should be 1")
	}
}
