// Package panicsafe centralizes crash logging and panic-safe execution helpers.
// It lives above logger/config (which it depends on) so any package can recover
// panics into ~/.sqyre/sqyre.log and optionally notify the user.
package panicsafe

import (
	"Sqyre/internal/config"
	"Sqyre/internal/logger"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime/debug"
	"time"
)

// OnPanicNotifyUser is called when a panic is recovered so the UI can show a dialog.
// Set by the UI package (e.g. to show dialog.ShowError). May be nil.
var OnPanicNotifyUser func(message string)

// LogPanicToFile writes the panic value and stack trace to ~/.sqyre/sqyre.log.
// Use this from recover() in any goroutine so that crashes are always recorded.
// It also invokes OnPanicNotifyUser so the user sees a notification.
// Optional context is prepended to the user message (e.g. "Macro \"foo\"").
func LogPanicToFile(r any, context ...string) {
	userMsg := fmt.Sprintf("Recovered from crash: %v", r)
	if len(context) > 0 && context[0] != "" {
		userMsg = context[0] + " — " + userMsg
	}
	stack := debug.Stack()
	logPath := filepath.Join(config.GetSqyreDir(), "sqyre.log")
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		logger.Errorf("panic (log file unavailable): %v", r)
		fmt.Fprintf(os.Stderr, "panic: %v\n%s", r, stack)
	} else {
		ts := time.Now().Format("2006-01-02 15:04:05.000")
		fmt.Fprintf(f, "\n[%s] panic recovered: %v\n", ts, r)
		if len(context) > 0 && context[0] != "" {
			fmt.Fprintf(f, "context: %s\n", context[0])
		}
		f.Write(stack)
		fmt.Fprintf(f, "\n")
		_ = f.Sync()
		_ = f.Close()
	}
	logger.Errorf("panic recovered: %v", r)
	log.Printf("panic recovered: %s", userMsg)
	log.Printf("panic stack:\n%s", stack)
	if OnPanicNotifyUser != nil {
		OnPanicNotifyUser(userMsg)
	}
}

// GoSafe runs fn in a new goroutine with panic recovery.
// Any panic is logged to the sqyre.log file and the user is notified.
func GoSafe(fn func()) {
	go func() {
		defer func() {
			if r := recover(); r != nil {
				LogPanicToFile(r)
			}
		}()
		fn()
	}()
}

// RunWithRecovery runs fn and recovers from any panic so the process does not exit.
// Logs the panic to file and notifies the user. Use to wrap app entry (e.g. main or app.Run).
func RunWithRecovery(fn func()) {
	defer func() {
		if r := recover(); r != nil {
			LogPanicToFile(r, "Application")
		}
	}()
	fn()
}

// SyncWriter wraps an *os.File and calls Sync after every Write so that
// log output is persisted immediately. This prevents log loss when the
// process is terminated by an unrecoverable crash (e.g. cgo/segfault).
type SyncWriter struct {
	F *os.File
}

func (w *SyncWriter) Write(p []byte) (int, error) {
	n, err := w.F.Write(p)
	_ = w.F.Sync()
	return n, err
}
