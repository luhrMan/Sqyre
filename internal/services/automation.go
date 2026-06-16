package services

import (
	"sync"

	"github.com/go-vgo/robotgo"
)

// AutomationBackend abstracts mouse, keyboard, timing, and clipboard operations
// so executor logic can be tested without real hardware input.
type AutomationBackend interface {
	MilliSleep(ms int)
	Move(x, y int, smooth bool)
	Click(button string, down bool) error
	KeyDown(key string) error
	KeyUp(key string) error
	TypeChar(s string)
	WriteClipboard(s string) error
}

type robotgoBackend struct{}

func (robotgoBackend) MilliSleep(ms int)              { robotgo.MilliSleep(ms) }
func (robotgoBackend) Move(x, y int, smooth bool)     { moveMouse(x, y, smooth) }
func (robotgoBackend) Click(button string, down bool) error {
	if down {
		return robotgo.Toggle(button)
	}
	return robotgo.Toggle(button, "up")
}
func (robotgoBackend) KeyDown(key string) error  { return robotgo.KeyDown(key) }
func (robotgoBackend) KeyUp(key string) error    { return robotgo.KeyUp(key) }
func (robotgoBackend) TypeChar(s string)         { robotgo.Type(s) }
func (robotgoBackend) WriteClipboard(s string) error { return robotgo.WriteAll(s) }

func moveMouse(x, y int, smooth bool) {
	if smooth {
		robotgo.MoveSmooth(x, y, 0.5, 1.01)
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

// RecordedCall is one automation operation captured by RecordingBackend.
type RecordedCall struct {
	Op     string
	Ms     int
	X, Y   int
	Smooth bool
	Button string
	Down   bool
	Key    string
	Char   string
	Text   string
}

// RecordingBackend records automation calls for assertions in tests.
type RecordingBackend struct {
	Calls []RecordedCall
}

func (r *RecordingBackend) MilliSleep(ms int) {
	r.Calls = append(r.Calls, RecordedCall{Op: "sleep", Ms: ms})
}

func (r *RecordingBackend) Move(x, y int, smooth bool) {
	r.Calls = append(r.Calls, RecordedCall{Op: "move", X: x, Y: y, Smooth: smooth})
}

func (r *RecordingBackend) Click(button string, down bool) error {
	r.Calls = append(r.Calls, RecordedCall{Op: "click", Button: button, Down: down})
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
