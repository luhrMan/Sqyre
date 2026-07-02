//go:build linux && !wayland

package recording

import (
	"image"
	"log"

	"Sqyre/internal/capture"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver"
	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xproto"
)

// positionFyneOverlayWindow moves a Fyne window to absBounds using X11
// ConfigureWindow on the underlying window handle. Windows are reparented to
// the root window so ConfigureWindow coordinates are true desktop absolutes.
func positionFyneOverlayWindow(win fyne.Window, absBounds image.Rectangle) {
	nw, ok := win.(driver.NativeWindow)
	if !ok {
		return
	}
	nw.RunNative(func(ctx any) {
		xctx, ok := ctx.(driver.X11WindowContext)
		if !ok || xctx.WindowHandle == 0 {
			return
		}
		conn, err := xgb.NewConn()
		if err != nil {
			return
		}
		defer conn.Close()
		wid := xproto.Window(xctx.WindowHandle)
		root := xproto.Setup(conn).DefaultScreen(conn).Root

		_ = xproto.ChangeWindowAttributes(conn, wid, xproto.CwOverrideRedirect, []uint32{1}).Check()
		_ = xproto.ReparentWindow(conn, wid, root, int16(absBounds.Min.X), int16(absBounds.Min.Y)).Check()

		cfgMask := uint16(xproto.ConfigWindowX | xproto.ConfigWindowY | xproto.ConfigWindowWidth | xproto.ConfigWindowHeight)
		cfgValues := []uint32{
			uint32(absBounds.Min.X),
			uint32(absBounds.Min.Y),
			uint32(absBounds.Dx()),
			uint32(absBounds.Dy()),
		}
		_ = xproto.ConfigureWindow(conn, wid, cfgMask, cfgValues).Check()
		_ = xproto.ConfigureWindow(conn, wid, xproto.ConfigWindowStackMode, []uint32{uint32(xproto.StackModeAbove)}).Check()
		_ = xproto.MapWindow(conn, wid)
		conn.Sync()

		if capture.OverlayDiagnosticsEnabled() {
			logOverlayRootGeometry(conn, wid, root, absBounds)
		}
	})
}

func logOverlayRootGeometry(conn *xgb.Conn, wid, root xproto.Window, requested image.Rectangle) {
	geom, gerr := xproto.GetGeometry(conn, xproto.Drawable(wid)).Reply()
	if gerr != nil || geom == nil {
		if gerr != nil {
			log.Printf("overlay x11 diag: geometry query failed: %v", gerr)
		}
		return
	}
	rootCoords, rerr := xproto.TranslateCoordinates(conn, wid, root, 0, 0).Reply()
	if rerr != nil || rootCoords == nil {
		log.Printf(
			"overlay x11 diag: requested=%v parent_rel=(%d,%d)-(%d,%d) translate_err=%v",
			requested,
			int(geom.X), int(geom.Y),
			int(geom.X)+int(geom.Width), int(geom.Y)+int(geom.Height),
			rerr,
		)
		return
	}
	log.Printf(
		"overlay x11 diag: requested=%v root_actual=(%d,%d)-(%d,%d) size=%dx%d",
		requested,
		int(rootCoords.DstX), int(rootCoords.DstY),
		int(rootCoords.DstX)+int(geom.Width), int(rootCoords.DstY)+int(geom.Height),
		int(geom.Width), int(geom.Height),
	)
}
