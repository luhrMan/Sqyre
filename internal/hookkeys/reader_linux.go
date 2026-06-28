//go:build linux && !nohook

package hookkeys

import (
	"fmt"
	"sort"

	"github.com/jezek/xgb"
	"github.com/jezek/xgb/xproto"
)

// NewReader opens an X11 connection and reads the keyboard map for pressed-key polling.
func NewReader() (Reader, error) {
	conn, err := xgb.NewConn()
	if err != nil {
		return nil, err
	}
	setup := xproto.Setup(conn)
	minKC := setup.MinKeycode
	maxKC := setup.MaxKeycode
	count := byte(maxKC - minKC + 1)
	reply, err := xproto.GetKeyboardMapping(conn, minKC, count).Reply()
	if err != nil {
		conn.Close()
		return nil, err
	}
	return &linuxReader{
		conn:              conn,
		minKeycode:        minKC,
		maxKeycode:        maxKC,
		keysymsPerKeycode: reply.KeysymsPerKeycode,
		keysyms:           reply.Keysyms,
	}, nil
}

type linuxReader struct {
	conn              *xgb.Conn
	minKeycode        xproto.Keycode
	maxKeycode        xproto.Keycode
	keysymsPerKeycode uint8
	keysyms           []xproto.Keysym
}

func (r *linuxReader) Close() {
	if r.conn != nil {
		r.conn.Close()
		r.conn = nil
	}
}

func (r *linuxReader) PressedKeyNames() []string {
	if r.conn == nil {
		return nil
	}
	keymap, err := xproto.QueryKeymap(r.conn).Reply()
	if err != nil {
		return nil
	}
	return namesFromKeymap(keymap.Keys, r.minKeycode, r.maxKeycode, r.keysyms, r.keysymsPerKeycode)
}

func namesFromKeymap(
	keymap []byte,
	minKeycode, maxKeycode xproto.Keycode,
	keysyms []xproto.Keysym,
	keysymsPerKeycode uint8,
) []string {
	seen := make(map[string]struct{})
	// Keycode is a uint8; iterate with int so maxKeycode 255 does not wrap.
	for kc := int(minKeycode); kc <= int(maxKeycode); kc++ {
		keycode := xproto.Keycode(kc)
		byteIdx := keycode / 8
		bit := keycode % 8
		if int(byteIdx) >= len(keymap) {
			break
		}
		if keymap[byteIdx]&(1<<bit) == 0 {
			continue
		}
		base := (kc - int(minKeycode)) * int(keysymsPerKeycode)
		for col := 0; col < int(keysymsPerKeycode); col++ {
			idx := base + col
			if idx < 0 || idx >= len(keysyms) {
				break
			}
			if keysyms[idx] == 0 {
				continue
			}
			if name, ok := keysymToHookName(keysyms[idx]); ok {
				seen[name] = struct{}{}
				break
			}
		}
	}
	out := make([]string, 0, len(seen))
	for name := range seen {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

// keysymToHookName maps X11 keysyms to gohook / macro hotkey key names.
func keysymToHookName(ks xproto.Keysym) (string, bool) {
	switch ks {
	case 0x0020:
		return "space", true
	case 0xff08:
		return "delete", true
	case 0xff09:
		return "tab", true
	case 0xff0d:
		return "enter", true
	case 0xff1b:
		return "esc", true
	case 0xff51:
		return "left", true
	case 0xff52:
		return "up", true
	case 0xff53:
		return "right", true
	case 0xff54:
		return "down", true
	case 0xffbe:
		return "f1", true
	case 0xffbf:
		return "f2", true
	case 0xffc0:
		return "f3", true
	case 0xffc1:
		return "f4", true
	case 0xffc2:
		return "f5", true
	case 0xffc3:
		return "f6", true
	case 0xffc4:
		return "f7", true
	case 0xffc5:
		return "f8", true
	case 0xffc6:
		return "f9", true
	case 0xffc7:
		return "f10", true
	case 0xffc8:
		return "f11", true
	case 0xffc9:
		return "f12", true
	case 0xffe1:
		return "shift", true
	case 0xffe2:
		return "rshift", true
	case 0xffe3:
		return "ctrl", true
	case 0xffe4:
		return "ctrl", true
	case 0xffe9:
		return "alt", true
	case 0xffea, 0xfe03:
		return "ralt", true
	case 0xffeb:
		return "cmd", true
	case 0xffec:
		return "rcmd", true
	case 0xffb0:
		return "num0", true
	case 0xffb1:
		return "num1", true
	case 0xffb2:
		return "num2", true
	case 0xffb3:
		return "num3", true
	case 0xffb4:
		return "num4", true
	case 0xffb5:
		return "num5", true
	case 0xffb6:
		return "num6", true
	case 0xffb7:
		return "num7", true
	case 0xffb8:
		return "num8", true
	case 0xffb9:
		return "num9", true
	case 0xffaa:
		return "num_asterisk", true
	case 0xffab:
		return "num_plus", true
	case 0xffad:
		return "num_minus", true
	case 0xffae:
		return "num_period", true
	case 0xffaf:
		return "num_slash", true
	case 0xff8d:
		return "num_enter", true
	}
	if ks >= 0x20 && ks <= 0x7e {
		return fmt.Sprintf("%c", rune(ks)), true
	}
	return "", false
}
