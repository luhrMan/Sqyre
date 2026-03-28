package assets

import "testing"

func TestEngTrainedDataEmbedded(t *testing.T) {
	data := EngTrainedData()
	if len(data) == 0 {
		t.Fatal("expected embedded eng.traineddata to be non-empty")
	}
}
