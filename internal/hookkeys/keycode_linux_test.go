//go:build linux && !nohook

package hookkeys

import (
	"testing"

	"github.com/jezek/xgb/xproto"
)

func TestKeycodeForHookName_rightArrow(t *testing.T) {
	const rightKeycode = 114
	minKC := xproto.Keycode(8)
	maxKC := xproto.Keycode(120)
	perKC := uint8(4)
	keysyms := make([]xproto.Keysym, int(maxKC-minKC+1)*int(perKC))
	idx := (rightKeycode - int(minKC)) * int(perKC)
	keysyms[idx] = 0xff53 // XK_Right

	kc, ok := keycodeForHookNameFromMapping(minKC, maxKC, keysyms, perKC, "right")
	if !ok || kc != xproto.Keycode(rightKeycode) {
		t.Fatalf("got keycode %d ok=%v, want %d", kc, ok, rightKeycode)
	}
}

func keycodeForHookNameFromMapping(
	minKC, maxKC xproto.Keycode,
	keysyms []xproto.Keysym,
	perKC uint8,
	name string,
) (xproto.Keycode, bool) {
	for kc := int(minKC); kc <= int(maxKC); kc++ {
		base := (kc - int(minKC)) * int(perKC)
		for col := 0; col < int(perKC); col++ {
			idx := base + col
			if idx >= len(keysyms) || keysyms[idx] == 0 {
				continue
			}
			hookName, ok := keysymToHookName(keysyms[idx])
			if ok && hookName == name {
				return xproto.Keycode(kc), true
			}
		}
	}
	return 0, false
}
