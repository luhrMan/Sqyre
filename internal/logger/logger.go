package logger

import (
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"sync"
	"time"
)

// Level represents log severity.
type Level int

const (
	Debug Level = iota
	Info
	Warn
	Error
)

const (
	prefixDebug = "[DEBUG]"
	prefixInfo  = "[INFO] "
	prefixWarn  = "[WARN] "
	prefixError = "[ERROR]"
)

var (
	mu        sync.Mutex
	minLevel  = Info
	stderrOut = log.New(os.Stderr, "", 0)
	fileOut   *log.Logger
	filePath  string
)

// SetLogFile enables writing logs to path (e.g. ~/.sqyre/sqyre.log).
// If path is empty or opening fails, only stderr is used.
func SetLogFile(path string) {
	mu.Lock()
	defer mu.Unlock()
	filePath = path
	if path == "" {
		fileOut = nil
		return
	}
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		fileOut = nil
		return
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		fileOut = nil
		return
	}
	fileOut = log.New(f, "", 0)
}

// SetLevel sets the minimum level to log (default Info).
func SetLevel(l Level) {
	mu.Lock()
	defer mu.Unlock()
	minLevel = l
}

func output(level Level, prefix, format string, a ...interface{}) {
	mu.Lock()
	if level < minLevel {
		mu.Unlock()
		return
	}
	msg := format
	if len(a) > 0 {
		msg = fmt.Sprintf(format, a...)
	}
	ts := time.Now().Format("2006-01-02 15:04:05.000")
	line := ts + " " + prefix + " " + msg
	stderrOut.Output(2, line)
	if fileOut != nil {
		_ = fileOut.Output(2, line)
	}
	mu.Unlock()
}

// Debugf logs at Debug level.
func Debugf(format string, a ...interface{}) { output(Debug, prefixDebug, format, a...) }

// Infof logs at Info level.
func Infof(format string, a ...interface{}) { output(Info, prefixInfo, format, a...) }

// Warnf logs at Warn level.
func Warnf(format string, a ...interface{}) { output(Warn, prefixWarn, format, a...) }

// Errorf logs at Error level.
func Errorf(format string, a ...interface{}) { output(Error, prefixError, format, a...) }

// Writer returns an io.Writer that writes log lines at Info level.
// Useful for redirecting standard log or other libraries.
func Writer() io.Writer {
	return &writer{}
}

type writer struct{}

func (w *writer) Write(p []byte) (n int, err error) {
	msg := string(p)
	if len(msg) > 0 && msg[len(msg)-1] == '\n' {
		msg = msg[:len(msg)-1]
	}
	Infof("%s", msg)
	return len(p), nil
}
