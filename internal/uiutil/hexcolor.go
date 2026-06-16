package uiutil

import (
	"fmt"
	"image/color"
	"strings"
)

// HexToColor parses "#rrggbb", "rrggbb", or "aarrggbb" into a color. Alpha is ignored for display.
func HexToColor(hex string) (color.Color, bool) {
	hex = strings.TrimPrefix(strings.ToLower(strings.TrimSpace(hex)), "#")
	if len(hex) == 8 {
		hex = hex[2:]
	}
	if len(hex) != 6 {
		return color.RGBA{}, false
	}
	var r, g, b uint8
	if _, err := fmt.Sscanf(hex, "%02x%02x%02x", &r, &g, &b); err != nil {
		return color.RGBA{}, false
	}
	return color.RGBA{R: r, G: g, B: b, A: 255}, true
}
