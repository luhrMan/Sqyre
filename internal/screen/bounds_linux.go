//go:build !js && linux

package screen

import (
	"image"

	"github.com/go-vgo/robotgo"
	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xinerama"
	"github.com/vcaesar/screenshot"
)

func displayBoundsAbsImpl(displayIndex int) image.Rectangle {
	if r := xineramaBounds(displayIndex); !r.Empty() {
		return r
	}
	// Wayland or no Xinerama: screenshot uses primary-relative rects; convert using primary origin when possible.
	rel := screenshot.GetDisplayBounds(displayIndex)
	if rel.Empty() {
		return image.Rectangle{}
	}
	if ox, oy, ok := xineramaPrimaryOrigin(); ok {
		return rel.Add(image.Pt(ox, oy))
	}
	return rel
}

func virtualBoundsImpl() image.Rectangle {
	if u := xineramaVirtualUnion(); !u.Empty() {
		return u
	}
	n := screenshot.NumActiveDisplays()
	if n <= 0 {
		w, h := robotgo.GetScreenSize()
		return image.Rect(0, 0, w, h)
	}
	var u image.Rectangle
	for i := 0; i < n; i++ {
		u = u.Union(displayBoundsAbsImpl(i))
	}
	if u.Empty() {
		w, h := robotgo.GetScreenSize()
		return image.Rect(0, 0, w, h)
	}
	return u
}

func xineramaBounds(displayIndex int) image.Rectangle {
	c, err := xgb.NewConn()
	if err != nil {
		return image.Rectangle{}
	}
	defer c.Close()
	if err := xinerama.Init(c); err != nil {
		return image.Rectangle{}
	}
	reply, err := xinerama.QueryScreens(c).Reply()
	if err != nil || displayIndex < 0 || displayIndex >= int(reply.Number) {
		return image.Rectangle{}
	}
	s := reply.ScreenInfo[displayIndex]
	x0, y0 := int(s.XOrg), int(s.YOrg)
	w, h := int(s.Width), int(s.Height)
	return image.Rect(x0, y0, x0+w, y0+h)
}

func xineramaVirtualUnion() image.Rectangle {
	c, err := xgb.NewConn()
	if err != nil {
		return image.Rectangle{}
	}
	defer c.Close()
	if err := xinerama.Init(c); err != nil {
		return image.Rectangle{}
	}
	reply, err := xinerama.QueryScreens(c).Reply()
	if err != nil || reply.Number == 0 {
		return image.Rectangle{}
	}
	var u image.Rectangle
	for i := 0; i < int(reply.Number); i++ {
		s := reply.ScreenInfo[i]
		x0, y0 := int(s.XOrg), int(s.YOrg)
		w, h := int(s.Width), int(s.Height)
		u = u.Union(image.Rect(x0, y0, x0+w, y0+h))
	}
	return u
}

func xineramaPrimaryOrigin() (ox int, oy int, ok bool) {
	c, err := xgb.NewConn()
	if err != nil {
		return 0, 0, false
	}
	defer c.Close()
	if err := xinerama.Init(c); err != nil {
		return 0, 0, false
	}
	reply, err := xinerama.QueryScreens(c).Reply()
	if err != nil || reply.Number == 0 {
		return 0, 0, false
	}
	p := reply.ScreenInfo[0]
	return int(p.XOrg), int(p.YOrg), true
}
