package actions

import (
	"fmt"
	"strconv"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// FindPixel scans a search area for a pixel matching the target color.
// When WaitTilFound is true it retries at WaitTilFoundIntervalMs intervals
// up to WaitTilFoundSeconds; sub-actions run when the pixel is found.
type FindPixel struct {
	SearchArea             SearchArea `mapstructure:"searcharea"`
	TargetColor            string     `mapstructure:"targetcolor"`
	ColorTolerance         int        `mapstructure:"colortolerance"`
	WaitTilFound           bool       `mapstructure:"waittilfound"`
	WaitTilFoundSeconds    int        `mapstructure:"waittilfoundseconds"`
	WaitTilFoundIntervalMs int        `mapstructure:"waittilfoundintervalms"`
	OutputXVariable        string     `mapstructure:"outputxvariable"`
	OutputYVariable        string     `mapstructure:"outputyvariable"`
	*AdvancedAction        `yaml:",inline" mapstructure:",squash"`
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

func NewFindPixel(name string, searchArea SearchArea, targetColor string, colorTolerance int, subActions []ActionInterface) *FindPixel {
	if colorTolerance < 0 {
		colorTolerance = 0
	}
	if colorTolerance > 100 {
		colorTolerance = 100
	}
	return &FindPixel{
		AdvancedAction: newAdvancedAction(name, "findpixel", subActions),
		SearchArea:     searchArea,
		TargetColor:    strings.ToLower(strings.TrimPrefix(targetColor, "#")),
		ColorTolerance: colorTolerance,
	}
}

func (a *FindPixel) String() string {
	return stringifyParams(a.parameters())
}

func (a *FindPixel) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *FindPixel) parameters() []actionParam {
	areaLabel := a.SearchArea.Name
	if areaLabel == "" {
		areaLabel = fmt.Sprintf("(%v,%v)-(%v,%v)", a.SearchArea.LeftX, a.SearchArea.TopY, a.SearchArea.RightX, a.SearchArea.BottomY)
	}
	mode := "instant"
	if a.WaitTilFound {
		mode = fmt.Sprintf("wait %ds", a.WaitTilFoundSeconds)
	}
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Color", a.TargetColor),
		newParam("Tolerance", fmt.Sprintf("%d%%", a.ColorTolerance)),
		newParam("Search Area", areaLabel),
		newParam("Wait", mode),
	}
}

func (a *FindPixel) Icon() fyne.Resource {
	return theme.ColorChromaticIcon()
}
