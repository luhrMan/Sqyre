//go:build linux && !nohook

package hookkeys

import (
	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xproto"
)

// KeycodeForHookName returns the X11 keycode for a gohook/macro hotkey name on this display.
func KeycodeForHookName(conn *xgb.Conn, name string) (xproto.Keycode, bool, error) {
	setup := xproto.Setup(conn)
	minKC := setup.MinKeycode
	maxKC := setup.MaxKeycode
	reply, err := xproto.GetKeyboardMapping(conn, minKC, byte(maxKC-minKC+1)).Reply()
	if err != nil {
		return 0, false, err
	}
	for kc := int(minKC); kc <= int(maxKC); kc++ {
		base := (kc - int(minKC)) * int(reply.KeysymsPerKeycode)
		for col := 0; col < int(reply.KeysymsPerKeycode); col++ {
			idx := base + col
			if idx >= len(reply.Keysyms) || reply.Keysyms[idx] == 0 {
				continue
			}
			hookName, ok := keysymToHookName(reply.Keysyms[idx])
			if ok && hookName == name {
				return xproto.Keycode(kc), true, nil
			}
		}
	}
	return 0, false, nil
}
