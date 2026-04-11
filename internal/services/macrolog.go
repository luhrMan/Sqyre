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
}

// StartMacroLogCapture begins capturing log output for the given macro.
// Logs are forwarded to onLine for display in the UI and buffered for initial display.
// Call StopMacroLogCapture when the macro finishes.
func StartMacroLogCapture(macroName string, onLine func(line string)) {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()

	if macroLogCapture != nil {
		return // already capturing
	}

	macroLogCapture = &macroLogCaptureState{
		buffer:   &bytes.Buffer{},
		original: log.Writer(),
	}

	// Custom writer: buffer + original (sqyre.log), and forward lines to onLine for UI
	w := &macroLogWriter{
		buffer:   macroLogCapture.buffer,
		original: macroLogCapture.original,
		onLine:   onLine,
	}
	log.SetOutput(w)
}

// StopMacroLogCapture stops capturing and restores the original log output.
func StopMacroLogCapture() {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()

	if macroLogCapture == nil {
		return
	}

	log.SetOutput(macroLogCapture.original)
	macroLogCapture = nil
}

// GetMacroLogBuffer returns the current log buffer content. Empty if not capturing.
func GetMacroLogBuffer() string {
	macroLogMu.Lock()
	defer macroLogMu.Unlock()
	if macroLogCapture == nil {
		return ""
	}
	return macroLogCapture.buffer.String()
}

// macroLogWriter writes to buffer and original log, and forwards each line to onLine for UI display.
type macroLogWriter struct {
	buffer   *bytes.Buffer
	original io.Writer
	onLine   func(line string)
}

func (w *macroLogWriter) Write(p []byte) (n int, err error) {
	n = len(p)
	// Write to buffer and original (sqyre.log)
	if w.buffer != nil {
		_, _ = w.buffer.Write(p)
	}
	if w.original != nil {
		_, _ = w.original.Write(p)
	}
	// Forward line to UI callback for display in dialog
	if w.onLine != nil && len(p) > 0 {
		text := strings.TrimSuffix(string(p), "\n")
		if text != "" {
			line := text
			RunOnMainThread(func() {
				w.onLine(line)
			})
		}
	}
	return n, nil
}
