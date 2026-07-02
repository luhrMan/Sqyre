package services

import (
	"sync"
	"time"

	"github.com/go-vgo/robotgo"
)

// AutomationBackend abstracts mouse, keyboard, timing, and clipboard operations
// so executor logic can be tested without real hardware input.
// MoveOptions configures mouse movement; Smooth=false ignores the other fields.
type MoveOptions struct {
	Smooth  bool
	Low     float64
	High    float64
	DelayMs int
}

type AutomationBackend interface {
	MilliSleep(ms int)
	Move(x, y int, opts MoveOptions)
	Click(button string, down bool) error
	Scroll(up bool) error
	KeyDown(key string) error
	KeyUp(key string) error
	TypeChar(s string)
	WriteClipboard(s string) error
}

type robotgoBackend struct{}

func (robotgoBackend) MilliSleep(ms int)              { robotgo.MilliSleep(ms) }
func (robotgoBackend) Move(x, y int, opts MoveOptions) { moveMouse(x, y, opts) }
func (robotgoBackend) Click(button string, down bool) error {
	if down {
		return robotgo.Toggle(button)
	}
	return robotgo.Toggle(button, "up")
}
func (robotgoBackend) Scroll(up bool) error {
	dir := "down"
	if up {
		dir = "up"
	}
	robotgo.ScrollDir(120, dir)
	return nil
}
func (robotgoBackend) KeyDown(key string) error  { return robotgo.KeyDown(key) }
func (robotgoBackend) KeyUp(key string) error    { return robotgo.KeyUp(key) }
func (robotgoBackend) TypeChar(s string)         { robotgo.Type(s) }
func (robotgoBackend) WriteClipboard(s string) error { return robotgo.WriteAll(s) }

func moveMouse(x, y int, opts MoveOptions) {
	if opts.Smooth {
		robotgo.MoveSmooth(x, y, opts.Low, opts.High, opts.DelayMs)
	} else {
		robotgo.Move(x, y)
	}
}

var (
	automationBackend AutomationBackend = robotgoBackend{}
	automationMu      sync.RWMutex
)

func getAutomationBackend() AutomationBackend {
	automationMu.RLock()
	defer automationMu.RUnlock()
	return automationBackend
}

// SetAutomationBackend replaces the global automation backend (tests only).
func SetAutomationBackend(b AutomationBackend) {
	automationMu.Lock()
	automationBackend = b
	automationMu.Unlock()
}

// ResetAutomationBackend restores the default robotgo backend.
func ResetAutomationBackend() {
	SetAutomationBackend(robotgoBackend{})
}

// mouseButtons is the set of mouse buttons that can be held down between click actions.
var mouseButtons = []string{"left", "right", "center"}

// ReleaseAllMouseButtons sends button-up for left, right, and center so no click
// remains physically held after a macro run ends.
func ReleaseAllMouseButtons() {
	backend := getAutomationBackend()
	for _, btn := range mouseButtons {
		_ = backend.Click(btn, false)
	}
}

// RecordedCall is one automation operation captured by RecordingBackend.
type RecordedCall struct {
	Op     string
	Ms     int
	X, Y   int
	Smooth        bool
	SmoothLow     float64
	SmoothHigh    float64
	SmoothDelayMs int
	Button string
	Down   bool
	Key    string
	Char   string
	Text   string
}

// RecordingBackend records automation calls for assertions in tests.
type RecordingBackend struct {
	Calls     []RecordedCall
	RealSleep bool
}

func (r *RecordingBackend) MilliSleep(ms int) {
	r.Calls = append(r.Calls, RecordedCall{Op: "sleep", Ms: ms})
	if r.RealSleep {
		time.Sleep(time.Duration(ms) * time.Millisecond)
	}
}

func (r *RecordingBackend) Move(x, y int, opts MoveOptions) {
	r.Calls = append(r.Calls, RecordedCall{
		Op:            "move",
		X:             x,
		Y:             y,
		Smooth:        opts.Smooth,
		SmoothLow:     opts.Low,
		SmoothHigh:    opts.High,
		SmoothDelayMs: opts.DelayMs,
	})
}

func (r *RecordingBackend) Click(button string, down bool) error {
	r.Calls = append(r.Calls, RecordedCall{Op: "click", Button: button, Down: down})
	return nil
}

func (r *RecordingBackend) Scroll(up bool) error {
	dir := "down"
	if up {
		dir = "up"
	}
	r.Calls = append(r.Calls, RecordedCall{Op: "scroll", Button: dir})
	return nil
}

func (r *RecordingBackend) KeyDown(key string) error {
	r.Calls = append(r.Calls, RecordedCall{Op: "keydown", Key: key})
	return nil
}

func (r *RecordingBackend) KeyUp(key string) error {
	r.Calls = append(r.Calls, RecordedCall{Op: "keyup", Key: key})
	return nil
}

func (r *RecordingBackend) TypeChar(s string) {
	r.Calls = append(r.Calls, RecordedCall{Op: "type", Char: s})
}

func (r *RecordingBackend) WriteClipboard(s string) error {
	r.Calls = append(r.Calls, RecordedCall{Op: "clipboard", Text: s})
	return nil
}
