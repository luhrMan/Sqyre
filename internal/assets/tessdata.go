package assets

import (
	_ "embed"
)

// engTrainedData is the English Tesseract language data. Embedded at build time;
// the build fails if internal/assets/tessdata/eng.traineddata is missing.
//go:embed tessdata/eng.traineddata
var engTrainedData []byte

// EngTrainedData returns the embedded English traineddata for in-memory Tesseract init (no disk write).
func EngTrainedData() []byte { return engTrainedData }
