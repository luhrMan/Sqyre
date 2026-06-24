//go:build linux && !wayland

package recording

import (
	"image"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver"
	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xproto"
)

// positionFyneOverlayWindow moves a Fyne window to absBounds using X11
// ConfigureWindow on the underlying window handle. This is queued via
// RunNative and may not take effect until the next event-loop tick.
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

		_ = xproto.ChangeWindowAttributes(conn, wid, xproto.CwOverrideRedirect, []uint32{1}).Check()

		cfgMask := uint16(xproto.ConfigWindowX | xproto.ConfigWindowY | xproto.ConfigWindowWidth | xproto.ConfigWindowHeight)
		cfgValues := []uint32{
			uint32(int32(absBounds.Min.X)),
			uint32(int32(absBounds.Min.Y)),
			uint32(absBounds.Dx()),
			uint32(absBounds.Dy()),
		}
		_ = xproto.ConfigureWindow(conn, wid, cfgMask, cfgValues).Check()
		_ = xproto.ConfigureWindow(conn, wid, xproto.ConfigWindowStackMode, []uint32{uint32(xproto.StackModeAbove)}).Check()
		conn.Sync()
	})
}
