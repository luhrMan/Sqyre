//go:build linux && !nohook

package macrohotkey

import (
	"fmt"
	"strings"

	"Sqyre/internal/hookkeys"
	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xproto"
)

var x11ModifierMask = map[string]uint16{
	"ctrl":  xproto.ModMaskControl,
	"shift": xproto.ModMaskShift,
	"alt":   xproto.ModMask1,
	"win":   xproto.ModMask4,
	"cmd":   xproto.ModMask4,
	"super": xproto.ModMask4,
}

// grabContinueChord optionally grabs the trigger key on X11 so other clients do not receive it.
func grabContinueChord(keys []string, grab bool) (func(), error) {
	if !grab {
		return func() {}, nil
	}
	conn, err := xgb.NewConn()
	if err != nil {
		return nil, err
	}
	root := xproto.Setup(conn).DefaultScreen(conn).Root

	var modMask uint16
	var trigger string
	for _, k := range keys {
		name := strings.ToLower(strings.TrimSpace(k))
		if m, ok := x11ModifierMask[name]; ok {
			modMask |= m
			continue
		}
		if trigger != "" {
			conn.Close()
			return nil, fmt.Errorf("pause: only one non-modifier key is supported for suppression")
		}
		trigger = name
	}
	if trigger == "" {
		conn.Close()
		return nil, fmt.Errorf("pause: continue key needs a non-modifier key for suppression")
	}
	kc, ok, err := hookkeys.KeycodeForHookName(conn, trigger)
	if err != nil {
		conn.Close()
		return nil, err
	}
	if !ok {
		conn.Close()
		return nil, fmt.Errorf("pause: unknown key %q", trigger)
	}
	keycode := kc
	if err := xproto.GrabKey(
		conn,
		true,
		root,
		modMask,
		keycode,
		xproto.GrabModeAsync,
		xproto.GrabModeAsync,
	).Check(); err != nil {
		conn.Close()
		return nil, err
	}
	return func() {
		_ = xproto.UngrabKey(conn, keycode, root, modMask).Check()
		conn.Close()
	}, nil
}
