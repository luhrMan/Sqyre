//go:build linux && !nohook

package hookkeys

import (
	"testing"

	"github.com/jezek/xgb/xproto"
)

func TestKeysymToHookName_arrows(t *testing.T) {
	cases := []struct {
		keysym xproto.Keysym
		want   string
	}{
		{0xff51, "left"},
		{0xff53, "right"},
		{0xff52, "up"},
		{0xff54, "down"},
	}
	for _, tc := range cases {
		got, ok := keysymToHookName(tc.keysym)
		if !ok || got != tc.want {
			t.Fatalf("keysym %#x: got %q ok=%v, want %q", tc.keysym, got, ok, tc.want)
		}
	}
}

func TestNamesFromKeymap_leftArrow(t *testing.T) {
	const leftKeycode = 113
	var keymap [32]byte
	keymap[leftKeycode/8] |= 1 << (leftKeycode % 8)

	minKC := xproto.Keycode(8)
	maxKC := xproto.Keycode(120)
	perKC := uint8(4)
	keysyms := make([]xproto.Keysym, int(maxKC-minKC+1)*int(perKC))
	idx := (leftKeycode - int(minKC)) * int(perKC)
	keysyms[idx] = 0xff51 // XK_Left

	got := namesFromKeymap(keymap[:], minKC, maxKC, keysyms, perKC)
	if len(got) != 1 || got[0] != "left" {
		t.Fatalf("got %v, want [left]", got)
	}
}

func TestNamesFromKeymap_loopDoesNotWrapAt255(t *testing.T) {
	var keymap [32]byte
	keymap[10] = 0x01 // keycode 80

	minKC := xproto.Keycode(8)
	maxKC := xproto.Keycode(255)
	perKC := uint8(1)
	keysyms := make([]xproto.Keysym, int(maxKC-minKC+1))
	keysyms[80-int(minKC)] = 'x'

	got := namesFromKeymap(keymap[:], minKC, maxKC, keysyms, perKC)
	if len(got) != 1 || got[0] != "x" {
		t.Fatalf("got %v, want [x]", got)
	}
}
