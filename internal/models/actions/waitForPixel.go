package actions

import (
	"fmt"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// WaitForPixel pauses macro playback until a specified display pixel changes to the target color,
// or until timeout; sub-actions run when the pixel is found. See https://www.macrorecorder.com/doc/wait/#pixel
type WaitForPixel struct {
	Point           Point  `mapstructure:"point"`
	TargetColor     string `mapstructure:"targetcolor"`    // Hex e.g. "ffffff" or "aarrggbb"
	ColorTolerance  int    `mapstructure:"colortolerance"` // 0-100%; 0 = exact match, 100 = any color
	TimeoutSeconds  int    `mapstructure:"timeoutseconds"` // 0 = wait indefinitely; on timeout, continue without running children
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

// NormalizeHex returns lowercase hex without alpha for comparison (robotgo returns 8-char hex on some platforms).
func (a *WaitForPixel) NormalizeHex(hex string) string {
	hex = strings.TrimPrefix(strings.ToLower(hex), "#")
	if len(hex) == 8 {
		hex = hex[2:] // drop alpha for comparison
	}
	return hex
}

// hexToRGB parses 6-char hex to r, g, b (0-255). Returns false if invalid.
func (a *WaitForPixel) hexToRGB(hex string) (r, g, b uint8, ok bool) {
	hex = a.NormalizeHex(hex)
	if len(hex) != 6 {
		return 0, 0, 0, false
	}
	rr, err1 := strconv.ParseUint(hex[0:2], 16, 8)
	gg, err2 := strconv.ParseUint(hex[2:4], 16, 8)
	bb, err3 := strconv.ParseUint(hex[4:6], 16, 8)
	if err1 != nil || err2 != nil || err3 != nil {
		return 0, 0, 0, false
	}
	return uint8(rr), uint8(gg), uint8(bb), true
}

// MatchColor returns true if screenHex matches the target color within ColorTolerance (0-100%).
// 0% = exact match; 100% = any color matches (per-channel delta up to 255).
func (a *WaitForPixel) MatchColor(screenHex string) bool {
	tr, tg, tb, tok := a.hexToRGB(a.TargetColor)
	sr, sg, sb, sok := a.hexToRGB(screenHex)
	if !tok || !sok {
		return a.NormalizeHex(screenHex) == a.NormalizeHex(a.TargetColor) // fallback exact
	}
	if a.ColorTolerance >= 100 {
		return true
	}
	if a.ColorTolerance <= 0 {
		return tr == sr && tg == sg && tb == sb
	}
	delta := uint8(255 * a.ColorTolerance / 100)
	if delta > 255 {
		delta = 255
	}
	dr := uint8(0)
	if tr > sr {
		dr = tr - sr
	} else {
		dr = sr - tr
	}
	dg := uint8(0)
	if tg > sg {
		dg = tg - sg
	} else {
		dg = sg - tg
	}
	db := uint8(0)
	if tb > sb {
		db = tb - sb
	} else {
		db = sb - tb
	}
	return dr <= delta && dg <= delta && db <= delta
}

func NewWaitForPixel(name string, point Point, targetColor string, colorTolerance int, timeoutSeconds int, subActions []ActionInterface) *WaitForPixel {
	if colorTolerance < 0 {
		colorTolerance = 0
	}
	if colorTolerance > 100 {
		colorTolerance = 100
	}
	return &WaitForPixel{
		AdvancedAction: newAdvancedAction(name, "waitforpixel", subActions),
		Point:          point,
		TargetColor:    strings.ToLower(strings.TrimPrefix(targetColor, "#")),
		ColorTolerance: colorTolerance,
		TimeoutSeconds: timeoutSeconds,
	}
}

func (a *WaitForPixel) String() string {
	if a.TimeoutSeconds > 0 {
		return fmt.Sprintf("%s --- Wait %ds at %v, %v for color #%s", a.Name, a.TimeoutSeconds, a.Point.X, a.Point.Y, a.TargetColor)
	}
	return fmt.Sprintf("%s --- Wait indefinitely at %v, %v for color #%s", a.Name, a.Point.X, a.Point.Y, a.TargetColor)
}

func (a *WaitForPixel) Icon() fyne.Resource {
	return theme.ColorChromaticIcon()
}
