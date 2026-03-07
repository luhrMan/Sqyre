package services

import (
	"Squire/internal/config"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime/debug"
	"time"
)

// LogPanicToFile writes the panic value and stack trace to ~/.sqyre/sqyre.log.
// Use this from recover() in any goroutine so that crashes are always recorded
// in the log file (panics in non-main goroutines are otherwise only printed to stderr).
func LogPanicToFile(r interface{}) {
	logPath := filepath.Join(config.GetSqyreDir(), "sqyre.log")
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		fmt.Fprintf(os.Stderr, "panic (log file unavailable): %v\n%s", r, debug.Stack())
		return
	}
	defer f.Close()
	ts := time.Now().Format("2006-01-02 15:04:05.000")
	fmt.Fprintf(f, "\n[%s] panic recovered: %v\n", ts, r)
	f.Write(debug.Stack())
	fmt.Fprintf(f, "\n")
	f.Sync()
}

// GoSafe runs fn in a new goroutine with panic recovery.
// Any panic is logged to the sqyre.log file and to the standard logger.
func GoSafe(fn func()) {
	go func() {
		defer func() {
			if r := recover(); r != nil {
				LogPanicToFile(r)
				log.Printf("goroutine recovered from panic: %v", r)
			}
		}()
		fn()
	}()
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
