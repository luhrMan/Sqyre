//go:build !gocv

package main

func runGocvSection(name string, _ uint64) sectionResult {
	return skipped(name, "needs -tags gocv with OpenCV matching gocv (this build used the gocv stub)")
}
