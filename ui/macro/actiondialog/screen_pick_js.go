//go:build js

package actiondialog

func screenPointerXY() (x, y int) {
	return 0, 0
}

func pixelColorHexAt(x, y int) string {
	_ = x
	_ = y
	return "808080"
}
