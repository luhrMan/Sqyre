package services

import (
	"bytes"
	"log"
	"os"
	"strings"
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func TestExecuteSemanticSearch_logs(t *testing.T) {
	t.Helper()
	os.Setenv("SQYRE_TEST_MODE", "1")
	dir := t.TempDir()
	configPath := dir + "/db.yaml"
	if err := os.WriteFile(configPath, []byte("macros: {}\nprograms: {}\n"), 0644); err != nil {
		t.Fatal(err)
	}
	os.Setenv("SQYRE_CONFIG", configPath)

	var buf bytes.Buffer
	log.SetOutput(&buf)
	t.Cleanup(func() { log.SetOutput(os.Stderr) })

	StartMacroLogCapture("test")
	t.Cleanup(StopMacroLogCapture)

	m := &models.Macro{Name: "m"}
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{
		actions.NewSemanticSearch("ss", nil, "button", ""),
	})
	if err := Execute(m.Root, m); err != nil {
		t.Fatalf("Execute: %v", err)
	}

	out := GetMacroLogBuffer()
	if !strings.Contains(out, "Semantic Search:") {
		t.Fatalf("macro log buffer missing Semantic Search line:\n%s", out)
	}
	if !strings.Contains(out, "Semantic search:") {
		t.Fatalf("macro log buffer missing vision-layer log line:\n%s", out)
	}
}
