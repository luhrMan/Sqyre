//go:build windows

package recording

import (
	"image"
	"syscall"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver"
)

var (
	user32             = syscall.NewLazyDLL("user32.dll")
	procSetWindowPos   = user32.NewProc("SetWindowPos")
	procGetWindowLongW = user32.NewProc("GetWindowLongW")
	procSetWindowLongW = user32.NewProc("SetWindowLongW")
)

const (
	gwlStyle   = ^uintptr(15)
	gwlExStyle = ^uintptr(19)

	wsCaption     = 0x00C00000
	wsThickFrame  = 0x00040000
	wsSysMenu     = 0x00080000
	wsMaximizeBox = 0x00010000
	wsMinimizeBox = 0x00020000

	wsExTopmost    = 0x00000008
	wsExToolWindow = 0x00000080

	hwndTopmost = ^uintptr(0)

	swpShowWindow   = 0x0040
	swpFrameChanged = 0x0020
)

func positionFyneOverlayWindow(win fyne.Window, absBounds image.Rectangle) {
	nw, ok := win.(driver.NativeWindow)
	if !ok {
		return
	}
	nw.RunNative(func(ctx any) {
		wctx, ok := ctx.(driver.WindowsWindowContext)
		if !ok || wctx.HWND == 0 {
			return
		}
		hwnd := wctx.HWND

		style, _, _ := procGetWindowLongW.Call(hwnd, gwlStyle)
		style &^= wsCaption | wsThickFrame | wsSysMenu | wsMaximizeBox | wsMinimizeBox
		procSetWindowLongW.Call(hwnd, gwlStyle, style)

		exStyle, _, _ := procGetWindowLongW.Call(hwnd, gwlExStyle)
		exStyle |= wsExTopmost | wsExToolWindow
		procSetWindowLongW.Call(hwnd, gwlExStyle, exStyle)

		procSetWindowPos.Call(
			hwnd,
			hwndTopmost,
			uintptr(absBounds.Min.X),
			uintptr(absBounds.Min.Y),
			uintptr(absBounds.Dx()),
			uintptr(absBounds.Dy()),
			uintptr(swpShowWindow|swpFrameChanged),
		)
	})
}
