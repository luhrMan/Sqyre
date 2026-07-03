package actions

import (
	"fmt"
	"strconv"
	"strings"

)

// FindPixel is a leaf (basic) action: it scans a search area for a pixel
// matching the target color and writes the match coordinates to variables.
// When WaitTilFound is true it retries at WaitTilFoundIntervalMs intervals
// up to WaitTilFoundSeconds. It no longer branches on whether the pixel was
// found — use a Conditional action on the output variable for true/false
// branching.
type FindPixel struct {
	Name           string
	SearchArea     CoordinateRef `mapstructure:"searcharea"`
	TargetColor    string        `mapstructure:"targetcolor"`
	ColorTolerance int           `mapstructure:"colortolerance"`
	WaitTilFoundConfig `yaml:",inline" mapstructure:",squash"`
	CoordinateOutputs  `yaml:",inline" mapstructure:",squash"`
	*BaseAction        `yaml:",inline" mapstructure:",squash"`
}

// NormalizeHex returns lowercase hex without alpha for comparison (robotgo returns 8-char hex on some platforms).
func (a *FindPixel) NormalizeHex(hex string) string {
	hex = strings.TrimPrefix(strings.ToLower(hex), "#")
	if len(hex) == 8 {
		hex = hex[2:]
	}
	return hex
}

func (a *FindPixel) hexToRGB(hex string) (r, g, b uint8, ok bool) {
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
func (a *FindPixel) MatchColor(screenHex string) bool {
	tr, tg, tb, tok := a.hexToRGB(a.TargetColor)
	sr, sg, sb, sok := a.hexToRGB(screenHex)
	if !tok || !sok {
		return a.NormalizeHex(screenHex) == a.NormalizeHex(a.TargetColor)
	}
	return a.matchRGB(tr, tg, tb, sr, sg, sb)
}

// matchRGB reports whether screen RGB (sr,sg,sb) matches target RGB (tr,tg,tb)
// within ColorTolerance (0-100%).
func (a *FindPixel) matchRGB(tr, tg, tb, sr, sg, sb uint8) bool {
	if a.ColorTolerance >= 100 {
		return true
	}
	if a.ColorTolerance <= 0 {
		return tr == sr && tg == sg && tb == sb
	}
	delta := uint8(255 * a.ColorTolerance / 100)
	return absDiffU8(tr, sr) <= delta && absDiffU8(tg, sg) <= delta && absDiffU8(tb, sb) <= delta
}

// ColorMatcher precomputes the target color and tolerance once and returns a
// closure that matches raw RGB bytes. Use this for hot pixel-scan loops to
// avoid per-pixel hex formatting and parsing. Returns a never-matching closure
// when the target color is not a valid hex value.
func (a *FindPixel) ColorMatcher() func(r, g, b uint8) bool {
	tr, tg, tb, tok := a.hexToRGB(a.TargetColor)
	if !tok {
		return func(_, _, _ uint8) bool { return false }
	}
	if a.ColorTolerance >= 100 {
		return func(_, _, _ uint8) bool { return true }
	}
	if a.ColorTolerance <= 0 {
		return func(r, g, b uint8) bool { return r == tr && g == tg && b == tb }
	}
	delta := uint8(255 * a.ColorTolerance / 100)
	return func(r, g, b uint8) bool {
		return absDiffU8(r, tr) <= delta && absDiffU8(g, tg) <= delta && absDiffU8(b, tb) <= delta
	}
}

func absDiffU8(a, b uint8) uint8 {
	if a > b {
		return a - b
	}
	return b - a
}

func NewFindPixel(name string, searchArea CoordinateRef, targetColor string, colorTolerance int) *FindPixel {
	if colorTolerance < 0 {
		colorTolerance = 0
	}
	if colorTolerance > 100 {
		colorTolerance = 100
	}
	return &FindPixel{
		BaseAction:     newBaseAction("findpixel"),
		Name:           name,
		SearchArea:     searchArea,
		TargetColor:    strings.ToLower(strings.TrimPrefix(targetColor, "#")),
		ColorTolerance: colorTolerance,
	}
}

func (a *FindPixel) String() string {
	return stringifyParams(a.Params())
}

func (a *FindPixel) Params() []Param {
	mode := a.WaitTilFoundConfig.DisplayWaitMode("instant")
	return []Param{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Color", a.TargetColor),
		newParam("Tolerance", fmt.Sprintf("%d%%", a.ColorTolerance)),
		newParam("Search Area", a.SearchArea.DisplayLabel()),
		newParam("Wait", mode),
	}
}

func (a *FindPixel) VariableBindings() []VariableBinding {
	return a.CoordinateOutputs.VariableBindings()
}
