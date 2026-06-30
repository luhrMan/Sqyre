package logger

import (
	"log"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestSetLogFileHooksStdLog(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "test.log")
	SetLogFile(path)
	t.Cleanup(func() { SetLogFile("") })

	log.SetFlags(0)
	log.Print("sqyre-log-hook-test")

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read log file: %v", err)
	}
	if !strings.Contains(string(data), "sqyre-log-hook-test") {
		t.Fatalf("log file missing message: %q", string(data))
	}
}
