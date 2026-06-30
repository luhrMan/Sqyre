package macro

import (
	"time"

	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
)

// The log pump decouples macro execution from the UI. Execution writes log lines
// and variable snapshots into non-blocking buffers (in package services); this
// pump drains them onto the UI thread at a fixed, modest rate. That keeps the
// Fyne event queue free for execution highlights and makes log rendering a
// constant-cost operation regardless of how fast the macro runs.

const logPumpInterval = 150 * time.Millisecond

var (
	activeLogContent *MacroTabContent // UI-thread only
	logPumpStop      chan struct{}    // UI-thread only
)

// setActiveLogContent marks which tab the pump should flush into. UI thread only.
func setActiveLogContent(c *MacroTabContent) {
	activeLogContent = c
}

// startLogPump begins periodic draining of buffered log lines and live variables.
// Safe to call repeatedly; a running pump is stopped first. UI thread only.
func startLogPump() {
	stopLogPump()
	stop := make(chan struct{})
	logPumpStop = stop
	go func() {
		t := time.NewTicker(logPumpInterval)
		defer t.Stop()
		for {
			select {
			case <-stop:
				fyne.Do(flushActiveLog) // final drain after the macro stops
				return
			case <-t.C:
				fyne.Do(flushActiveLog)
			}
		}
	}()
}

// stopLogPump halts the pump (a final flush is queued by the goroutine). UI thread only.
func stopLogPump() {
	if logPumpStop != nil {
		close(logPumpStop)
		logPumpStop = nil
	}
}

// flushActiveLog drains pending log lines and refreshes live variables. UI thread only.
func flushActiveLog() {
	c := activeLogContent
	if c == nil {
		return
	}
	c.appendDrainedLog(services.DrainMacroLogLines())
	c.updateLiveVars()
}
