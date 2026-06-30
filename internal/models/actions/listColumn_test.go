package actions

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestListColumn_manualText_LineCount_GetCurrentLine_NextLine_Reset(t *testing.T) {
	c := ListColumn{Source: "a\nb\nc", OutputVar: "v"}
	n, err := c.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 3 {
		t.Errorf("LineCount() = %d, want 3", n)
	}
	first, err := c.GetCurrentLine()
	if err != nil || first != "a" {
		t.Errorf("GetCurrentLine() = %q, err=%v", first, err)
	}
	c.NextLine()
	line, _ := c.GetCurrentLine()
	if line != "b" {
		t.Errorf("after NextLine GetCurrentLine() = %q", line)
	}
	c.NextLine()
	line, _ = c.GetCurrentLine()
	if line != "c" {
		t.Errorf("GetCurrentLine() = %q", line)
	}
	c.NextLine()
	line, _ = c.GetCurrentLine()
	if line != first {
		t.Errorf("after wrap GetCurrentLine() = %q, want first line %q", line, first)
	}
	c.Reset()
	line, _ = c.GetCurrentLine()
	if line != first {
		t.Errorf("after Reset GetCurrentLine() = %q", line)
	}
}

func TestListColumn_manualText_trailingNewline(t *testing.T) {
	c := ListColumn{Source: "x\ny\n", OutputVar: "v"}
	n, err := c.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 2 {
		t.Errorf("trailing newline should give 2 lines, got %d", n)
	}
}

func TestListColumn_manualText_skipBlankLines(t *testing.T) {
	c := ListColumn{Source: "a\n\nb\n  \nc", OutputVar: "v", SkipBlankLines: true}
	n, err := c.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 3 {
		t.Errorf("LineCount() with SkipBlankLines = %d, want 3", n)
	}
}

func TestListColumn_file_absolutePath(t *testing.T) {
	dir := t.TempDir()
	f := filepath.Join(dir, "lines.txt")
	if err := os.WriteFile(f, []byte("first\nsecond\nthird\n"), 0644); err != nil {
		t.Fatal(err)
	}
	c := ListColumn{Source: f, OutputVar: "v", IsFile: true}
	n, err := c.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 3 {
		t.Errorf("LineCount() = %d, want 3", n)
	}
	line, err := c.GetCurrentLine()
	if err != nil || line != "first" {
		t.Errorf("GetCurrentLine() = %q, err=%v", line, err)
	}
}

func TestListColumn_GetCurrentLine_outOfRange(t *testing.T) {
	c := ListColumn{Source: "only", OutputVar: "v"}
	_, _ = c.LineCount()
	_, err := c.GetCurrentLine()
	if err != nil {
		t.Fatal(err)
	}
	c.currentLine = 5
	_, err = c.GetCurrentLine()
	if err == nil {
		t.Error("expected error when currentLine out of range")
	}
}

func TestListColumn_LineCount_fileNotFound(t *testing.T) {
	c := ListColumn{Source: "nonexistent-file-actions-test-12345.txt", OutputVar: "v", IsFile: true}
	_, err := c.LineCount()
	if err == nil {
		t.Error("LineCount() with missing file should return error")
	}
}

func TestListColumn_sourceChangeReloads(t *testing.T) {
	dir := t.TempDir()
	f1 := filepath.Join(dir, "first.txt")
	f2 := filepath.Join(dir, "second.txt")
	if err := os.WriteFile(f1, []byte("old-line\n"), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(f2, []byte("new-line\n"), 0644); err != nil {
		t.Fatal(err)
	}

	c := ListColumn{Source: f1, OutputVar: "v", IsFile: true}
	line, err := c.GetCurrentLine()
	if err != nil || line != "old-line" {
		t.Fatalf("first read = %q, err=%v", line, err)
	}

	c.Source = f2
	line, err = c.GetCurrentLine()
	if err != nil || line != "new-line" {
		t.Fatalf("after source change = %q, err=%v, want new-line", line, err)
	}
}

func TestListColumn_ResetReloadsFromDisk(t *testing.T) {
	dir := t.TempDir()
	f := filepath.Join(dir, "lines.txt")
	if err := os.WriteFile(f, []byte("first\n"), 0644); err != nil {
		t.Fatal(err)
	}

	c := ListColumn{Source: f, OutputVar: "v", IsFile: true}
	line, err := c.GetCurrentLine()
	if err != nil || line != "first" {
		t.Fatalf("first read = %q, err=%v", line, err)
	}

	if err := os.WriteFile(f, []byte("updated\n"), 0644); err != nil {
		t.Fatal(err)
	}
	c.Reset()
	line, err = c.GetCurrentLine()
	if err != nil || line != "updated" {
		t.Fatalf("after reset = %q, err=%v, want updated", line, err)
	}
}

func TestForEachRow_String(t *testing.T) {
	fer := NewForEachRow("rows", []ListColumn{{Source: "a\nb", OutputVar: "x"}}, nil)
	if got := fer.String(); !strings.Contains(got, "foreachrow") || !strings.Contains(got, "rows") {
		t.Errorf("String() = %q", got)
	}
}
