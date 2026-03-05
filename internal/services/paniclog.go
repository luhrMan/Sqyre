package services

import (
	"Squire/internal/config"
	"fmt"
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
		// Fallback to stderr so something is visible
		fmt.Fprintf(os.Stderr, "panic (log file unavailable): %v\n%s", r, debug.Stack())
		return
	}
	defer f.Close()
	ts := time.Now().Format("2006-01-02 15:04:05.000")
	fmt.Fprintf(f, "\n[%s] panic recovered: %v\n", ts, r)
	f.Write(debug.Stack())
	fmt.Fprintf(f, "\n")
}
