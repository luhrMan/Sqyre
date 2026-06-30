package services

import (
	"bytes"
	"io"
	"log"
	"strings"
	"sync"
)

var (
	macroLogMu      sync.Mutex
	macroLogCapture *macroLogCaptureState
)

type macroLogCaptureState struct {
	buffer   *bytes.Buffer
	original io.Writer
	pending  []string // lines not yet drained by the UI
}

const maxMacroLogBufferBytes = 512 * 1024
const maxMacroLogPendingLines = 400

// StartMacroLogCapture begins capturing log output for the given macro.
//
// Lines are buffered (for Copy) and appended to a pending queue that the UI
// drains on its own schedule via DrainMacroLogLines. Crucially, capture does NOT
// touch the UI thread per line: a fast macro can emit thousands of log lines
// without flooding the Fyne event queue (which would also delay highlight
// updates). Call StopMacroLogCapture when the macro finishes.
func StartMacroLogCapture(macroName string) {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()

	macroLogCapture = &macroLogCaptureState{
		buffer:   &bytes.Buffer{},
		original: log.Writer(),
	}
	log.SetOutput(&macroLogWriter{original: macroLogCapture.original})
}

// ReleaseMacroLogCapture drops retained log buffers after a macro finishes.
// The UI keeps its own trimmed view; this frees backend memory held for Copy.
func ReleaseMacroLogCapture() {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()
	if macroLogCapture == nil {
		return
	}
	macroLogCapture.buffer = nil
	macroLogCapture.pending = nil
}

// StopMacroLogCapture stops capturing and restores the original log output.
// The captured buffer and any undrained pending lines are retained so the UI can
// perform a final drain (and so Copy still works) until the next capture starts.
func StopMacroLogCapture() {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()

	if macroLogCapture == nil {
		return
	}
	log.SetOutput(macroLogCapture.original)
}

// DrainMacroLogLines returns and clears the lines accumulated since the last
// drain. Returns nil when there is nothing new.
func DrainMacroLogLines() []string {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()
	if macroLogCapture == nil || len(macroLogCapture.pending) == 0 {
		return nil
	}
	lines := macroLogCapture.pending
	macroLogCapture.pending = nil
	return lines
}

// GetMacroLogBuffer returns the full captured log text (empty if never captured).
func GetMacroLogBuffer() string {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()
	if macroLogCapture == nil || macroLogCapture.buffer == nil {
		return ""
	}
	return macroLogCapture.buffer.String()
}

// macroLogWriter writes to the original log sink (sqyre.log) and accumulates
// captured lines for the UI to drain. It never calls into the UI directly.
type macroLogWriter struct {
	original io.Writer
}

func (w *macroLogWriter) Write(p []byte) (n int, err error) {
	n = len(p)
	if w.original != nil {
		_, _ = w.original.Write(p)
	}
	macroLogMu.Lock()
	if macroLogCapture != nil {
		if macroLogCapture.buffer != nil && macroLogCapture.buffer.Len() < maxMacroLogBufferBytes {
			_, _ = macroLogCapture.buffer.Write(p)
		}
		text := strings.TrimSuffix(string(p), "\n")
		if text != "" && len(macroLogCapture.pending) < maxMacroLogPendingLines {
			macroLogCapture.pending = append(macroLogCapture.pending, text)
		}
	}
	macroLogMu.Unlock()
	return n, nil
}
