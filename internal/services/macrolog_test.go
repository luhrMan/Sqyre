package services

import (
	"log"
	"strings"
	"testing"
	"time"
)

func TestMacroLogCapture_Lifecycle(t *testing.T) {
	var captured []string
	StartMacroLogCapture("test", func(line string) {
		captured = append(captured, line)
	})

	log.Println("hello from macro")
	// RunOnMainThread defaults to synchronous, so callback fires immediately
	time.Sleep(10 * time.Millisecond)

	buf := GetMacroLogBuffer()
	if !strings.Contains(buf, "hello from macro") {
		t.Errorf("buffer missing log line, got: %q", buf)
	}
	if len(captured) == 0 {
		t.Error("onLine callback was never called")
	}

	StopMacroLogCapture()

	if GetMacroLogBuffer() != "" {
		t.Error("buffer should be empty after stop")
	}
}

func TestMacroLogCapture_DoubleStart(t *testing.T) {
	StartMacroLogCapture("first", nil)
	StartMacroLogCapture("second", nil) // should be no-op
	StopMacroLogCapture()
	StopMacroLogCapture() // double stop is safe
}
