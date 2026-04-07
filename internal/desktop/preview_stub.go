//go:build sqyre_no_desktop_native

package desktop

import "image"

// MonitorOutline describes one display for dotted-outline preview drawing.
type MonitorOutline struct {
	AbsBounds image.Rectangle
	Enabled   bool
}

func SearchAreaPreviewImage(image.Rectangle, int, int, int, int, []MonitorOutline) (image.Image, error) {
	return nil, ErrUnavailable
}

func PointPreviewImage(image.Rectangle, int, int, []MonitorOutline) (image.Image, error) {
	return nil, ErrUnavailable
}

func MaskImageFromFile(string) (image.Image, error) {
	return nil, ErrUnavailable
}
