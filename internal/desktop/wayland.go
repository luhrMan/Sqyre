//go:build !sqyre_no_desktop_native

package desktop

import (
	"context"
	"encoding/json"
	"fmt"
	"image"
	"image/draw"
	"image/png"
	"log"
	"math"
	"net/url"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"sync"
	"time"

	_ "image/jpeg"

	"github.com/bnema/libwldevices-go/virtual_keyboard"
	"github.com/bnema/libwldevices-go/virtual_pointer"
	"github.com/rymdport/portal/screenshot"
)

// waylandInput holds lazy-initialized Wayland virtual devices.
var waylandInput struct {
	pointerOnce  sync.Once
	keyboardOnce sync.Once
	pointer      *virtual_pointer.VirtualPointer
	pointerMgr   *virtual_pointer.VirtualPointerManager
	keyboard     *virtual_keyboard.VirtualKeyboard
	keyboardMgr  *virtual_keyboard.VirtualKeyboardManager
}

func getWaylandPointer() *virtual_pointer.VirtualPointer {
	waylandInput.pointerOnce.Do(func() {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		mgr, err := virtual_pointer.NewVirtualPointerManager(ctx)
		if err != nil {
			log.Printf("wayland: virtual pointer manager: %v", err)
			return
		}
		ptr, err := mgr.CreatePointer()
		if err != nil {
			log.Printf("wayland: create virtual pointer: %v", err)
			mgr.Close()
			return
		}
		waylandInput.pointerMgr = mgr
		waylandInput.pointer = ptr
	})
	return waylandInput.pointer
}

func getWaylandKeyboard() *virtual_keyboard.VirtualKeyboard {
	waylandInput.keyboardOnce.Do(func() {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		mgr, err := virtual_keyboard.NewVirtualKeyboardManager(ctx)
		if err != nil {
			log.Printf("wayland: virtual keyboard manager: %v", err)
			return
		}
		kb, err := mgr.CreateKeyboard()
		if err != nil {
			log.Printf("wayland: create virtual keyboard: %v", err)
			mgr.Close()
			return
		}
		waylandInput.keyboardMgr = mgr
		waylandInput.keyboard = kb
	})
	return waylandInput.keyboard
}

type waylandBridge struct{}

func (waylandBridge) Location() (int, int) {
	if x, y, ok := hyprctlCursorPos(); ok {
		return x, y
	}
	return 0, 0
}

func (waylandBridge) MilliSleep(ms int) {
	time.Sleep(time.Duration(ms) * time.Millisecond)
}

func (waylandBridge) CaptureImg(x, y, w, h int) (image.Image, error) {
	return portalCapture(x, y, w, h)
}

func (waylandBridge) SavePng(img image.Image, path string) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return png.Encode(f, img)
}

func (wb waylandBridge) PixelColorHex(x, y int) string {
	img, err := wb.CaptureImg(x, y, 1, 1)
	if err != nil {
		return "808080"
	}
	b := img.Bounds()
	r, g, bl, _ := img.At(b.Min.X, b.Min.Y).RGBA()
	return fmt.Sprintf("%02x%02x%02x", r>>8, g>>8, bl>>8)
}

func (waylandBridge) ProcessID() int { return os.Getpid() }

func (waylandBridge) WindowBounds(int) (int, int, int, int) { return 0, 0, 0, 0 }

func (waylandBridge) SetMouseSleep(int) {}

func (waylandBridge) SetKeySleep(int) {}

// ---------- Input automation ----------

func (wb waylandBridge) Move(x, y int) {
	ptr := getWaylandPointer()
	if ptr == nil {
		return
	}
	// Use MotionAbsolute via the virtual pointer protocol.
	// xExtent/yExtent define the coordinate space; the compositor maps
	// (x, y) within (0..xExtent, 0..yExtent) to the output layout.
	// We use a large fixed extent and scale the target coordinates into it.
	const extent = 0xFFFF
	scaledX := uint32(float64(x) / float64(screenTotalWidth()) * float64(extent))
	scaledY := uint32(float64(y) / float64(screenTotalHeight()) * float64(extent))
	if err := ptr.MotionAbsolute(time.Now(), scaledX, scaledY, extent, extent); err != nil {
		log.Printf("wayland Move: %v", err)
		return
	}
	_ = ptr.Frame()
}

func (wb waylandBridge) MoveSmooth(x, y int, low, high float64) {
	curX, curY := wb.Location()
	dx := float64(x - curX)
	dy := float64(y - curY)
	dist := math.Hypot(dx, dy)

	steps := int(dist / 4)
	if steps < 2 {
		steps = 2
	}
	if steps > 200 {
		steps = 200
	}

	for i := 1; i <= steps; i++ {
		t := float64(i) / float64(steps)
		ix := curX + int(dx*t)
		iy := curY + int(dy*t)
		wb.Move(ix, iy)
		time.Sleep(2 * time.Millisecond)
	}
}

func (waylandBridge) MouseToggle(btn string, downUp ...string) {
	ptr := getWaylandPointer()
	if ptr == nil {
		return
	}
	var btnCode uint32
	switch strings.ToLower(btn) {
	case "right":
		btnCode = virtual_pointer.BTN_RIGHT
	case "middle", "center":
		btnCode = virtual_pointer.BTN_MIDDLE
	default:
		btnCode = virtual_pointer.BTN_LEFT
	}
	state := virtual_pointer.ButtonStatePressed
	if len(downUp) > 0 && strings.ToLower(downUp[0]) == "up" {
		state = virtual_pointer.ButtonStateReleased
	}
	if err := ptr.Button(time.Now(), btnCode, state); err != nil {
		log.Printf("wayland MouseToggle: %v", err)
		return
	}
	_ = ptr.Frame()
}

func (waylandBridge) KeyDown(key string) error {
	kb := getWaylandKeyboard()
	if kb == nil {
		return fmt.Errorf("wayland keyboard unavailable")
	}
	code, ok := robotgoKeyToEvdev[strings.ToLower(key)]
	if !ok {
		return fmt.Errorf("wayland KeyDown: unknown key %q", key)
	}
	return kb.PressKey(code)
}

func (waylandBridge) KeyUp(key string) error {
	kb := getWaylandKeyboard()
	if kb == nil {
		return fmt.Errorf("wayland keyboard unavailable")
	}
	code, ok := robotgoKeyToEvdev[strings.ToLower(key)]
	if !ok {
		return fmt.Errorf("wayland KeyUp: unknown key %q", key)
	}
	return kb.ReleaseKey(code)
}

func (waylandBridge) TypeChar(s string) {
	kb := getWaylandKeyboard()
	if kb == nil {
		return
	}
	if err := kb.TypeString(s); err != nil {
		log.Printf("wayland TypeChar: %v", err)
	}
}

func (waylandBridge) ClipboardWrite(text string) {
	cmd := exec.Command("wl-copy", "--", text)
	if err := cmd.Run(); err != nil {
		log.Printf("wayland ClipboardWrite: wl-copy: %v", err)
	}
}

// ---------- Window management (Hyprland / Sway) ----------

func hyprlandSession() bool {
	return os.Getenv("HYPRLAND_INSTANCE_SIGNATURE") != ""
}

func swaySession() bool {
	return os.Getenv("SWAYSOCK") != ""
}

func (waylandBridge) FindWindowNames() ([]string, error) {
	switch {
	case hyprlandSession():
		return findWindowNamesHyprland()
	case swaySession():
		return findWindowNamesSway()
	default:
		return nil, fmt.Errorf("window listing on Wayland requires Hyprland (hyprctl) or Sway (swaymsg); other compositors are not supported yet")
	}
}

func findWindowNamesHyprland() ([]string, error) {
	cmd := exec.Command("hyprctl", "clients", "-j")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("hyprctl clients: %w: %s", err, strings.TrimSpace(string(out)))
	}
	var clients []struct {
		Class string `json:"class"`
		Title string `json:"title"`
	}
	if err := json.Unmarshal(out, &clients); err != nil {
		return nil, fmt.Errorf("hyprctl clients JSON: %w", err)
	}
	seen := make(map[string]bool)
	var names []string
	for _, c := range clients {
		name := c.Class
		if name == "" {
			name = c.Title
		}
		name = strings.TrimSpace(name)
		if name == "" || seen[name] {
			continue
		}
		seen[name] = true
		names = append(names, name)
	}
	return names, nil
}

func findWindowNamesSway() ([]string, error) {
	out, err := exec.Command("swaymsg", "-t", "get_tree").Output()
	if err != nil {
		return nil, fmt.Errorf("swaymsg get_tree: %w", err)
	}
	var root interface{}
	if err := json.Unmarshal(out, &root); err != nil {
		return nil, fmt.Errorf("sway get_tree JSON: %w", err)
	}
	seen := make(map[string]bool)
	var names []string
	var walk func(v interface{})
	walk = func(v interface{}) {
		switch x := v.(type) {
		case map[string]interface{}:
			if n := swayPickWindowName(x); n != "" && !seen[n] {
				seen[n] = true
				names = append(names, n)
			}
			for _, key := range []string{"nodes", "floating_nodes"} {
				if arr, ok := x[key].([]interface{}); ok {
					for _, n := range arr {
						walk(n)
					}
				}
			}
		case []interface{}:
			for _, n := range x {
				walk(n)
			}
		}
	}
	walk(root)
	return names, nil
}

func swayPickWindowName(x map[string]interface{}) string {
	if aid, ok := x["app_id"].(string); ok {
		if s := strings.TrimSpace(aid); s != "" {
			return s
		}
	}
	if wp, ok := x["window_properties"].(map[string]interface{}); ok {
		if cl, ok := wp["class"].(string); ok {
			if s := strings.TrimSpace(cl); s != "" {
				return s
			}
		}
	}
	if n, ok := x["name"].(string); ok {
		if s := strings.TrimSpace(n); s != "" {
			return s
		}
	}
	return ""
}

func swayCriteriaEscape(s string) string {
	s = strings.ReplaceAll(s, `\`, `\\`)
	s = strings.ReplaceAll(s, `"`, `\"`)
	return s
}

func (waylandBridge) ActiveWindowByName(name string) error {
	name = strings.TrimSpace(name)
	if name == "" {
		return fmt.Errorf("empty window target")
	}
	switch {
	case hyprlandSession():
		return exec.Command("hyprctl", "dispatch", "focuswindow", name).Run()
	case swaySession():
		return swayFocusByName(name)
	default:
		return fmt.Errorf("focus window on Wayland requires Hyprland or Sway")
	}
}

func swayFocusByName(name string) error {
	esc := swayCriteriaEscape(name)
	tries := []string{
		fmt.Sprintf(`[app_id="%s"] focus`, esc),
		fmt.Sprintf(`[class="%s"] focus`, esc),
	}
	var last error
	for _, crit := range tries {
		err := exec.Command("swaymsg", crit).Run()
		if err == nil {
			return nil
		}
		last = err
	}
	return fmt.Errorf("sway focus: %w", last)
}

// ---------- helpers ----------

// screenTotalWidth/Height return the total virtual desktop size for
// MotionAbsolute extent calculation. Falls back to 1920x1080.
func screenTotalWidth() int {
	out, err := exec.Command("hyprctl", "monitors", "-j").Output()
	if err != nil {
		return 1920
	}
	var monitors []struct {
		X      int `json:"x"`
		Width  int `json:"width"`
		Y      int `json:"y"`
		Height int `json:"height"`
	}
	if err := json.Unmarshal(out, &monitors); err != nil || len(monitors) == 0 {
		return 1920
	}
	maxX := 0
	for _, m := range monitors {
		if end := m.X + m.Width; end > maxX {
			maxX = end
		}
	}
	if maxX <= 0 {
		return 1920
	}
	return maxX
}

func screenTotalHeight() int {
	out, err := exec.Command("hyprctl", "monitors", "-j").Output()
	if err != nil {
		return 1080
	}
	var monitors []struct {
		X      int `json:"x"`
		Width  int `json:"width"`
		Y      int `json:"y"`
		Height int `json:"height"`
	}
	if err := json.Unmarshal(out, &monitors); err != nil || len(monitors) == 0 {
		return 1080
	}
	maxY := 0
	for _, m := range monitors {
		if end := m.Y + m.Height; end > maxY {
			maxY = end
		}
	}
	if maxY <= 0 {
		return 1080
	}
	return maxY
}

// portalCapture uses the XDG Desktop Portal Screenshot D-Bus API.
func portalCapture(x, y, w, h int) (image.Image, error) {
	uri, err := screenshot.Screenshot("", &screenshot.ScreenshotOptions{
		Interactive: false,
		NotModal:    true,
	})
	if err != nil {
		return nil, fmt.Errorf("portal screenshot: %w", err)
	}
	if uri == "" {
		return nil, fmt.Errorf("portal screenshot: cancelled or empty result")
	}

	u, err := url.Parse(uri)
	if err != nil {
		return nil, fmt.Errorf("portal screenshot: bad URI %q: %w", uri, err)
	}
	path := u.Path
	defer os.Remove(path)

	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	fullImg, _, err := image.Decode(f)
	if err != nil {
		return nil, err
	}

	cropRect := image.Rect(x, y, x+w, y+h).Intersect(fullImg.Bounds())
	if cropRect.Empty() {
		return nil, fmt.Errorf("portal screenshot: crop region (%d,%d %dx%d) outside image bounds %v",
			x, y, w, h, fullImg.Bounds())
	}

	cropped := image.NewRGBA(image.Rect(0, 0, cropRect.Dx(), cropRect.Dy()))
	draw.Draw(cropped, cropped.Bounds(), fullImg, cropRect.Min, draw.Src)
	return cropped, nil
}

func hyprctlCursorPos() (x, y int, ok bool) {
	out, err := exec.Command("hyprctl", "cursorpos").Output()
	if err != nil {
		return 0, 0, false
	}
	parts := strings.SplitN(strings.TrimSpace(string(out)), ",", 2)
	if len(parts) != 2 {
		return 0, 0, false
	}
	x, err1 := strconv.Atoi(strings.TrimSpace(parts[0]))
	y, err2 := strconv.Atoi(strings.TrimSpace(parts[1]))
	if err1 != nil || err2 != nil {
		return 0, 0, false
	}
	return x, y, true
}

// robotgoKeyToEvdev maps robotgo key names to Linux evdev keycodes.
var robotgoKeyToEvdev = map[string]uint32{
	// Letters
	"a": virtual_keyboard.KEY_A, "b": virtual_keyboard.KEY_B,
	"c": virtual_keyboard.KEY_C, "d": virtual_keyboard.KEY_D,
	"e": virtual_keyboard.KEY_E, "f": virtual_keyboard.KEY_F,
	"g": virtual_keyboard.KEY_G, "h": virtual_keyboard.KEY_H,
	"i": virtual_keyboard.KEY_I, "j": virtual_keyboard.KEY_J,
	"k": virtual_keyboard.KEY_K, "l": virtual_keyboard.KEY_L,
	"m": virtual_keyboard.KEY_M, "n": virtual_keyboard.KEY_N,
	"o": virtual_keyboard.KEY_O, "p": virtual_keyboard.KEY_P,
	"q": virtual_keyboard.KEY_Q, "r": virtual_keyboard.KEY_R,
	"s": virtual_keyboard.KEY_S, "t": virtual_keyboard.KEY_T,
	"u": virtual_keyboard.KEY_U, "v": virtual_keyboard.KEY_V,
	"w": virtual_keyboard.KEY_W, "x": virtual_keyboard.KEY_X,
	"y": virtual_keyboard.KEY_Y, "z": virtual_keyboard.KEY_Z,

	// Digits
	"0": virtual_keyboard.KEY_0, "1": virtual_keyboard.KEY_1,
	"2": virtual_keyboard.KEY_2, "3": virtual_keyboard.KEY_3,
	"4": virtual_keyboard.KEY_4, "5": virtual_keyboard.KEY_5,
	"6": virtual_keyboard.KEY_6, "7": virtual_keyboard.KEY_7,
	"8": virtual_keyboard.KEY_8, "9": virtual_keyboard.KEY_9,

	// Modifiers
	"shift": virtual_keyboard.KEY_LEFTSHIFT, "lshift": virtual_keyboard.KEY_LEFTSHIFT,
	"rshift": virtual_keyboard.KEY_RIGHTSHIFT,
	"ctrl":  virtual_keyboard.KEY_LEFTCTRL, "lctrl": virtual_keyboard.KEY_LEFTCTRL,
	"alt":   virtual_keyboard.KEY_LEFTALT, "lalt": virtual_keyboard.KEY_LEFTALT,
	"cmd":   virtual_keyboard.KEY_LEFTMETA, "lcmd": virtual_keyboard.KEY_LEFTMETA,
	"super": virtual_keyboard.KEY_LEFTMETA,

	// Special keys
	"enter":     virtual_keyboard.KEY_ENTER, "return": virtual_keyboard.KEY_ENTER,
	"tab":       virtual_keyboard.KEY_TAB,
	"space":     virtual_keyboard.KEY_SPACE,
	"backspace": virtual_keyboard.KEY_BACKSPACE,
	"escape":    virtual_keyboard.KEY_ESC, "esc": virtual_keyboard.KEY_ESC,
	"capslock":  virtual_keyboard.KEY_CAPSLOCK,

	// Punctuation / symbols
	"minus": virtual_keyboard.KEY_MINUS, "-": virtual_keyboard.KEY_MINUS,
	"equal": virtual_keyboard.KEY_EQUAL, "=": virtual_keyboard.KEY_EQUAL,
	"[": virtual_keyboard.KEY_LEFTBRACE, "]": virtual_keyboard.KEY_RIGHTBRACE,
	";": virtual_keyboard.KEY_SEMICOLON, "'": virtual_keyboard.KEY_APOSTROPHE,
	"`": virtual_keyboard.KEY_GRAVE, "\\": virtual_keyboard.KEY_BACKSLASH,
	",": virtual_keyboard.KEY_COMMA, ".": virtual_keyboard.KEY_DOT,
	"/": virtual_keyboard.KEY_SLASH,

	// Arrow keys (evdev codes)
	"up": 103, "down": 108, "left": 105, "right": 106,

	// Navigation
	"home": 102, "end": 107,
	"pageup": 104, "pagedown": 109,
	"insert": 110, "delete": 111,

	// Function keys
	"f1": 59, "f2": 60, "f3": 61, "f4": 62,
	"f5": 63, "f6": 64, "f7": 65, "f8": 66,
	"f9": 67, "f10": 68, "f11": 87, "f12": 88,

	// Numpad
	"num0": 82, "num1": 79, "num2": 80, "num3": 81,
	"num4": 75, "num5": 76, "num6": 77,
	"num7": 71, "num8": 72, "num9": 73,
	"num.": 83, "num+": 78, "num-": 74,
	"num*": 55, "num/": 98, "numlock": 69,

	// Media / misc
	"printscreen": 99, "scrolllock": 70, "pause": 119,
}
