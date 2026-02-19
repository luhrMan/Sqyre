package actions

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// --- BaseAction & interfaces ---

func TestBaseAction_GetType(t *testing.T) {
	a := newBaseAction("click")
	if a.GetType() != "click" {
		t.Errorf("GetType() = %q, want %q", a.GetType(), "click")
	}
}

func TestBaseAction_GetUID(t *testing.T) {
	a := newBaseAction("wait")
	uid := a.GetUID()
	if uid == "" {
		t.Error("GetUID() should not be empty")
	}
	// UIDs should be unique
	b := newBaseAction("wait")
	if b.GetUID() == uid {
		t.Error("GetUID() should be unique per action")
	}
}

func TestBaseAction_GetParent_SetParent(t *testing.T) {
	parent := newAdvancedAction("p", "loop", nil)
	child := NewWait(100)
	if child.GetParent() != nil {
		t.Error("new action should have nil parent")
	}
	child.SetParent(parent)
	if child.GetParent() != parent {
		t.Error("GetParent() should return set parent")
	}
}

func TestAdvancedAction_GetAction_AddSubAction_RemoveSubAction(t *testing.T) {
	sub := NewWait(50)
	adv := newAdvancedAction("adv", "loop", []ActionInterface{sub})
	sub.SetParent(adv)

	uid := sub.GetUID()
	got := adv.GetAction(uid)
	if got != sub {
		t.Errorf("GetAction(%q) = %v, want sub", uid, got)
	}
	if adv.GetAction("nonexistent") != nil {
		t.Error("GetAction(nonexistent) should return nil")
	}
	// GetAction on self
	if adv.GetAction(adv.GetUID()) != adv {
		t.Error("GetAction(own UID) should return self")
	}

	// AddSubAction
	sub2 := NewWait(100)
	adv.AddSubAction(sub2)
	if len(adv.GetSubActions()) != 2 {
		t.Errorf("after AddSubAction, len(GetSubActions()) = %d, want 2", len(adv.GetSubActions()))
	}
	if sub2.GetParent() != adv {
		t.Error("AddSubAction should set parent")
	}

	// RemoveSubAction
	adv.RemoveSubAction(sub)
	if len(adv.GetSubActions()) != 1 {
		t.Errorf("after RemoveSubAction, len(GetSubActions()) = %d, want 1", len(adv.GetSubActions()))
	}
	if adv.GetSubActions()[0] != sub2 {
		t.Error("remaining sub-action should be sub2")
	}
}

func TestAdvancedAction_SetSubActions(t *testing.T) {
	adv := newAdvancedAction("a", "loop", []ActionInterface{NewWait(1), NewWait(2)})
	if len(adv.GetSubActions()) != 2 {
		t.Fatalf("initial SubActions len = %d", len(adv.GetSubActions()))
	}
	newSubs := []ActionInterface{NewClick(false, false)}
	adv.SetSubActions(newSubs)
	if len(adv.GetSubActions()) != 1 || adv.GetSubActions()[0] != newSubs[0] {
		t.Error("SetSubActions did not update correctly")
	}
}

func TestBaseAction_String_Icon(t *testing.T) {
	b := newBaseAction("custom")
	if got := b.String(); got != "This is a baseAction" {
		t.Errorf("BaseAction.String() = %q", got)
	}
	if b.Icon() == nil {
		t.Error("BaseAction.Icon() should not be nil")
	}
}

func TestAdvancedAction_String(t *testing.T) {
	adv := newAdvancedAction("myname", "loop", nil)
	if got := adv.String(); got != "Advanced Action: loop" {
		t.Errorf("AdvancedAction.String() = %q", got)
	}
}

func TestAdvancedAction_GetAction_nested(t *testing.T) {
	inner := NewWait(1)
	innerLoop := newAdvancedAction("inner", "loop", []ActionInterface{inner})
	inner.SetParent(innerLoop)
	outer := newAdvancedAction("outer", "loop", []ActionInterface{innerLoop})
	innerLoop.SetParent(outer)
	uid := inner.GetUID()
	got := outer.GetAction(uid)
	if got != inner {
		t.Errorf("GetAction(nested UID) = %v, want inner", got)
	}
}

// --- Click ---

func TestNewClick(t *testing.T) {
	c := NewClick(false, false)
	if c.GetType() != "click" {
		t.Errorf("Type = %q", c.GetType())
	}
	if c.Button != false || c.Hold != false {
		t.Errorf("Button=%v Hold=%v", c.Button, c.Hold)
	}
}

func TestClick_String(t *testing.T) {
	if got := NewClick(false, false).String(); got != "left click" {
		t.Errorf("String() = %q", got)
	}
	if got := NewClick(true, false).String(); got != "right click" {
		t.Errorf("String() = %q", got)
	}
	if got := NewClick(false, true).String(); got != "left click (hold)" {
		t.Errorf("String() = %q", got)
	}
}

func TestLeftOrRight(t *testing.T) {
	if LeftOrRight(false) != "left" || LeftOrRight(true) != "right" {
		t.Error("LeftOrRight mismatch")
	}
}

func TestClick_Icon(t *testing.T) {
	if NewClick(false, false).Icon() == nil {
		t.Error("Icon() left click should not be nil")
	}
	if NewClick(false, true).Icon() == nil {
		t.Error("Icon() hold should not be nil")
	}
}

// --- Wait ---

func TestNewWait(t *testing.T) {
	w := NewWait(500)
	if w.GetType() != "wait" || w.Time != 500 {
		t.Errorf("Type=%q Time=%d", w.GetType(), w.Time)
	}
}

func TestWait_String(t *testing.T) {
	if got := NewWait(100).String(); got != "100 ms" {
		t.Errorf("String() = %q", got)
	}
}

// --- Move ---

func TestNewMove(t *testing.T) {
	p := Point{Name: "center", X: 100, Y: 200}
	m := NewMove(p)
	if m.GetType() != "move" || m.Point.Name != "center" || m.Point.X != 100 || m.Point.Y != 200 {
		t.Errorf("Move: %+v", m)
	}
}

func TestMove_String(t *testing.T) {
	m := NewMove(Point{Name: "A", X: 1, Y: 2})
	if got := m.String(); got != "A (1, 2)" {
		t.Errorf("String() = %q", got)
	}
}

// --- Key ---

func TestNewKey(t *testing.T) {
	k := NewKey("a", true)
	if k.GetType() != "key" || k.Key != "a" || k.State != true {
		t.Errorf("Key: %+v", k)
	}
}

func TestKey_String(t *testing.T) {
	if got := NewKey("x", false).String(); !strings.Contains(got, "x") || !strings.Contains(got, "up") {
		t.Errorf("String() = %q", got)
	}
	if got := NewKey("y", true).String(); !strings.Contains(got, "down") {
		t.Errorf("String() = %q", got)
	}
}

func TestUpOrDown(t *testing.T) {
	if UpOrDown(false) != "up" || UpOrDown(true) != "down" {
		t.Error("UpOrDown mismatch")
	}
}

// --- SetVariable ---

func TestNewSetVariable(t *testing.T) {
	sv := NewSetVariable("x", 42)
	if sv.GetType() != "setvariable" || sv.VariableName != "x" || sv.Value != 42 {
		t.Errorf("SetVariable: %+v", sv)
	}
}

func TestSetVariable_String(t *testing.T) {
	if got := NewSetVariable("foo", "bar").String(); got != "Set foo = bar" {
		t.Errorf("String() = %q", got)
	}
}

// --- SaveVariable ---

func TestNewSaveVariable(t *testing.T) {
	s := NewSaveVariable("v", "/path", true, true)
	if s.GetType() != "savevariable" || s.VariableName != "v" || s.Destination != "/path" || !s.Append || !s.AppendNewline {
		t.Errorf("SaveVariable: %+v", s)
	}
}

func TestSaveVariable_String(t *testing.T) {
	if got := NewSaveVariable("x", "clipboard", false, false).String(); got != "Save x to clipboard" {
		t.Errorf("String() = %q", got)
	}
	if got := NewSaveVariable("x", "/f", true, false).String(); got != "Append x to /f" {
		t.Errorf("String() = %q", got)
	}
	if got := NewSaveVariable("x", "/f", false, false).String(); got != "Save x to /f" {
		t.Errorf("String() = %q", got)
	}
}

func TestSaveVariable_SaveToFile(t *testing.T) {
	dir := t.TempDir()
	f := filepath.Join(dir, "out.txt")

	// Overwrite
	sv := NewSaveVariable("v", f, false, false)
	if err := sv.SaveToFile("hello", f); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(f)
	if string(data) != "hello" {
		t.Errorf("file content = %q", data)
	}

	// Append without newline
	sv2 := NewSaveVariable("v", f, true, false)
	if err := sv2.SaveToFile("world", f); err != nil {
		t.Fatal(err)
	}
	data, _ = os.ReadFile(f)
	if string(data) != "helloworld" {
		t.Errorf("after append = %q", data)
	}

	// Append with newline
	f2 := filepath.Join(dir, "out2.txt")
	_ = os.WriteFile(f2, []byte("first"), 0644)
	sv3 := NewSaveVariable("v", f2, true, true)
	if err := sv3.SaveToFile("second", f2); err != nil {
		t.Fatal(err)
	}
	data, _ = os.ReadFile(f2)
	if string(data) != "first\nsecond" {
		t.Errorf("append newline = %q", data)
	}
}

// --- Calculate ---

func TestNewCalculate(t *testing.T) {
	c := NewCalculate("1+1", "result")
	if c.GetType() != "calculate" || c.Expression != "1+1" || c.OutputVar != "result" {
		t.Errorf("Calculate: %+v", c)
	}
}

func TestCalculate_String(t *testing.T) {
	if got := NewCalculate("x+y", "z").String(); got != "Calculate: x+y -> z" {
		t.Errorf("String() = %q", got)
	}
}

// --- FocusWindow ---

func TestNewFocusWindow(t *testing.T) {
	f := NewFocusWindow("chrome")
	if f.GetType() != "focuswindow" || f.WindowTarget != "chrome" {
		t.Errorf("FocusWindow: %+v", f)
	}
}

func TestFocusWindow_String(t *testing.T) {
	if got := NewFocusWindow("code").String(); got != "Focus: code" {
		t.Errorf("String() = %q", got)
	}
	if got := NewFocusWindow("").String(); got != "Focus window (not set)" {
		t.Errorf("String() = %q", got)
	}
}

// --- Loop ---

func TestNewLoop(t *testing.T) {
	l := NewLoop(5, "myloop", nil)
	if l.GetType() != "loop" || l.Name != "myloop" || l.Count != 5 {
		t.Errorf("Loop: %+v", l)
	}
}

func TestNewLoop_root(t *testing.T) {
	l := NewLoop(nil, "root", nil)
	if l.Name != "root" || l.Count != 1 {
		t.Errorf("root Loop: Name=%q Count=%v", l.Name, l.Count)
	}
	if l.GetUID() != "" {
		t.Errorf("root Loop UID should be empty, got %q", l.GetUID())
	}
	if l.GetParent() != nil {
		t.Error("root Loop parent should be nil")
	}
}

func TestNewLoop_nilCount(t *testing.T) {
	l := NewLoop(nil, "x", nil)
	if l.Count != 1 {
		t.Errorf("nil count should default to 1, got %v", l.Count)
	}
}

func TestLoop_String(t *testing.T) {
	l := NewLoop(3, "L", nil)
	if got := l.String(); got != "L | iterations: 3" {
		t.Errorf("String() = %q", got)
	}
}

// --- Ocr ---

func TestNewOcr(t *testing.T) {
	area := SearchArea{Name: "box", LeftX: 0, TopY: 0, RightX: 100, BottomY: 100}
	o := NewOcr("myocr", nil, "target", area)
	if o.GetType() != "ocr" || o.Name != "myocr" || o.Target != "target" || o.SearchArea.Name != "box" {
		t.Errorf("Ocr: %+v", o)
	}
	if o.Blur != 3 || o.MinThreshold != 50 || o.Resize != 1.0 || !o.Grayscale {
		t.Errorf("Ocr defaults: Blur=%d MinThreshold=%d Resize=%v Grayscale=%v", o.Blur, o.MinThreshold, o.Resize, o.Grayscale)
	}
}

func TestOcr_String(t *testing.T) {
	area := SearchArea{Name: "A"}
	o := NewOcr("O", nil, "text", area)
	if got := o.String(); !strings.Contains(got, "O") || !strings.Contains(got, "text") || !strings.Contains(got, "A") || !strings.Contains(got, "instant") {
		t.Errorf("String() = %q", got)
	}
	o.WaitTilFound = true
	o.WaitTilFoundSeconds = 10
	if got := o.String(); !strings.Contains(got, "10 seconds") {
		t.Errorf("String() = %q", got)
	}
}

// --- ImageSearch ---

func TestNewImageSearch(t *testing.T) {
	area := SearchArea{Name: "region"}
	targets := []string{"b.png", "a.png"}
	is := NewImageSearch("im", nil, targets, area, 2, 3, 0.9, 1)
	if is.GetType() != "imagesearch" || is.Name != "im" || is.RowSplit != 2 || is.ColSplit != 3 || is.Tolerance != 0.9 || is.Blur != 1 {
		t.Errorf("ImageSearch: %+v", is)
	}
	// targets should be sorted
	if len(is.Targets) != 2 || is.Targets[0] != "a.png" || is.Targets[1] != "b.png" {
		t.Errorf("Targets = %v", is.Targets)
	}
}

func TestImageSearch_String(t *testing.T) {
	area := SearchArea{Name: "R"}
	is := NewImageSearch("S", nil, []string{"a"}, area, 1, 1, 0, 0)
	if got := is.String(); !strings.Contains(got, "1 items") || !strings.Contains(got, "R") {
		t.Errorf("String() = %q", got)
	}
	is.WaitTilFound = true
	is.WaitTilFoundSeconds = 5
	if got := is.String(); !strings.Contains(got, "5 seconds") {
		t.Errorf("String() = %q", got)
	}
}

// --- WaitForPixel ---

func TestNewWaitForPixel(t *testing.T) {
	p := Point{Name: "p", X: 10, Y: 20}
	w := NewWaitForPixel("wp", p, "#ff0000", 5, 10, nil)
	if w.GetType() != "waitforpixel" || w.Name != "wp" || w.TimeoutSeconds != 10 || w.ColorTolerance != 5 {
		t.Errorf("WaitForPixel: %+v", w)
	}
	if w.TargetColor != "ff0000" {
		t.Errorf("TargetColor = %q", w.TargetColor)
	}
}

func TestNewWaitForPixel_clampTolerance(t *testing.T) {
	p := Point{}
	w := NewWaitForPixel("w", p, "#000", -1, 0, nil)
	if w.ColorTolerance != 0 {
		t.Errorf("ColorTolerance should be clamped to 0, got %d", w.ColorTolerance)
	}
	w2 := NewWaitForPixel("w2", p, "#000", 150, 0, nil)
	if w2.ColorTolerance != 100 {
		t.Errorf("ColorTolerance should be clamped to 100, got %d", w2.ColorTolerance)
	}
}

func TestWaitForPixel_NormalizeHex(t *testing.T) {
	w := &WaitForPixel{}
	tests := []struct{ in, want string }{
		{"#FF00FF", "ff00ff"},
		{"ff00ff", "ff00ff"},
		{"aabbccdd", "bbccdd"},
		{"#AABBCCDD", "bbccdd"},
	}
	for _, tt := range tests {
		if got := w.NormalizeHex(tt.in); got != tt.want {
			t.Errorf("NormalizeHex(%q) = %q, want %q", tt.in, got, tt.want)
		}
	}
}

func TestWaitForPixel_MatchColor(t *testing.T) {
	w := &WaitForPixel{TargetColor: "ffffff", ColorTolerance: 0}
	if !w.MatchColor("#ffffff") {
		t.Error("exact match should be true")
	}
	if w.MatchColor("#000000") {
		t.Error("different color with 0 tolerance should be false")
	}
	w.ColorTolerance = 100
	// 100% tolerance returns true only when both colors parse as hex
	if !w.MatchColor("#000000") {
		t.Error("100% tolerance should match any valid hex")
	}
	w.ColorTolerance = 50
	w.TargetColor = "808080"
	// 80 hex = 128; 128+64=192 = c0, 128-64=64 = 40. So 40-c0 range per channel.
	if !w.MatchColor("c0c0c0") {
		t.Error("within tolerance should match")
	}
}

func TestWaitForPixel_String(t *testing.T) {
	p := Point{X: 1, Y: 2}
	w := NewWaitForPixel("W", p, "ff0000", 0, 0, nil)
	if got := w.String(); !strings.Contains(got, "indefinitely") || !strings.Contains(got, "ff0000") {
		t.Errorf("String() = %q", got)
	}
	w.TimeoutSeconds = 5
	if got := w.String(); !strings.Contains(got, "5") {
		t.Errorf("String() = %q", got)
	}
}

// --- DataList ---

func TestNewDataList(t *testing.T) {
	d := NewDataList("line1\nline2", "out", false)
	if d.GetType() != "datalist" || d.Source != "line1\nline2" || d.OutputVar != "out" || d.IsFile != false {
		t.Errorf("DataList: %+v", d)
	}
}

func TestDataList_manualText_LineCount_GetCurrentLine_NextLine_Reset(t *testing.T) {
	d := NewDataList("a\nb\nc", "v", false)
	n, err := d.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 3 {
		t.Errorf("LineCount() = %d, want 3", n)
	}
	first, err := d.GetCurrentLine()
	if err != nil || first != "a" {
		t.Errorf("GetCurrentLine() = %q, err=%v", first, err)
	}
	d.NextLine()
	line, _ := d.GetCurrentLine()
	if line != "b" {
		t.Errorf("after NextLine GetCurrentLine() = %q", line)
	}
	d.NextLine()
	line, _ = d.GetCurrentLine()
	if line != "c" {
		t.Errorf("GetCurrentLine() = %q", line)
	}
	d.NextLine() // wrap from last line back to first
	line, _ = d.GetCurrentLine()
	if line != first {
		t.Errorf("after wrap GetCurrentLine() = %q, want first line %q", line, first)
	}
	d.Reset()
	line, _ = d.GetCurrentLine()
	if line != first {
		t.Errorf("after Reset GetCurrentLine() = %q", line)
	}
}

func TestDataList_manualText_trailingNewline(t *testing.T) {
	d := NewDataList("x\ny\n", "v", false)
	n, err := d.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 2 {
		t.Errorf("trailing newline should give 2 lines, got %d", n)
	}
}

func TestDataList_manualText_skipBlankLines(t *testing.T) {
	d := NewDataList("a\n\nb\n  \nc", "v", false)
	d.SkipBlankLines = true
	n, err := d.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 3 {
		t.Errorf("LineCount() with SkipBlankLines = %d, want 3", n)
	}
	line, _ := d.GetCurrentLine()
	if line != "a" {
		t.Errorf("GetCurrentLine() = %q", line)
	}
	d.NextLine()
	line, _ = d.GetCurrentLine()
	if line != "b" {
		t.Errorf("GetCurrentLine() = %q", line)
	}
	d.NextLine()
	line, _ = d.GetCurrentLine()
	if line != "c" {
		t.Errorf("GetCurrentLine() = %q", line)
	}
}

func TestDataList_file_absolutePath(t *testing.T) {
	dir := t.TempDir()
	f := filepath.Join(dir, "lines.txt")
	if err := os.WriteFile(f, []byte("first\nsecond\nthird\n"), 0644); err != nil {
		t.Fatal(err)
	}
	d := NewDataList(f, "v", true)
	// Force file path to be used as absolute so we don't depend on config.GetVariablesPath
	d.Source = f
	n, err := d.LineCount()
	if err != nil {
		t.Fatal(err)
	}
	if n != 3 {
		t.Errorf("LineCount() = %d, want 3", n)
	}
	line, err := d.GetCurrentLine()
	if err != nil || line != "first" {
		t.Errorf("GetCurrentLine() = %q, err=%v", line, err)
	}
}

func TestDataList_GetCurrentLine_outOfRange(t *testing.T) {
	d := NewDataList("only", "v", false)
	_, _ = d.LineCount()
	_, err := d.GetCurrentLine()
	if err != nil {
		t.Fatal(err)
	}
	d.currentLine = 5
	_, err = d.GetCurrentLine()
	if err == nil {
		t.Error("expected error when currentLine out of range")
	}
}

func TestDataList_String(t *testing.T) {
	d := NewDataList("a\nb", "out", false)
	_, _ = d.LineCount()
	if got := d.String(); !strings.Contains(got, "2 lines") || !strings.Contains(got, "out") {
		t.Errorf("String() = %q", got)
	}
	d2 := NewDataList("/path/file", "v", true)
	if got := d2.String(); !strings.Contains(got, "file") || !strings.Contains(got, "/path/file") {
		t.Errorf("String() = %q", got)
	}
}

// TestActionTypes_Icon ensures Icon() is called on every action type for coverage.
func TestActionTypes_Icon(t *testing.T) {
	actions := []struct {
		name string
		a    ActionInterface
	}{
		{"Calculate", NewCalculate("1", "x")},
		{"DataList", NewDataList("x", "v", false)},
		{"FocusWindow", NewFocusWindow("w")},
		{"ImageSearch", NewImageSearch("s", nil, nil, SearchArea{}, 0, 0, 0, 0)},
		{"Key", NewKey("k", false)},
		{"KeyDown", NewKey("k", true)},
		{"Loop", NewLoop(1, "l", nil)},
		{"Move", NewMove(Point{})},
		{"Ocr", NewOcr("o", nil, "", SearchArea{})},
		{"SaveVariable", NewSaveVariable("v", "d", false, false)},
		{"SetVariable", NewSetVariable("v", 0)},
		{"Wait", NewWait(0)},
		{"WaitForPixel", NewWaitForPixel("w", Point{}, "000000", 0, 0, nil)},
	}
	for _, tt := range actions {
		t.Run(tt.name, func(t *testing.T) {
			if tt.a.Icon() == nil {
				t.Error("Icon() should not be nil")
			}
		})
	}
}

func TestDataList_LineCount_fileNotFound(t *testing.T) {
	d := NewDataList("nonexistent-file-actions-test-12345.txt", "v", true)
	// IsFile true + non-absolute path -> loadLines joins with config path and reads; file missing
	_, err := d.LineCount()
	if err == nil {
		t.Error("LineCount() with missing file should return error")
	}
}

func TestDataList_GetCurrentLine_loadError(t *testing.T) {
	d := NewDataList("nonexistent-getcurrentline-12345.txt", "v", true)
	// GetCurrentLine loads on demand; missing file causes loadLines to fail
	_, err := d.GetCurrentLine()
	if err == nil {
		t.Error("GetCurrentLine() with missing file should return error")
	}
}

func TestSaveVariable_SaveToFile_openError(t *testing.T) {
	dir := t.TempDir()
	badPath := filepath.Join(dir, "missing", "sub", "file.txt") // parent does not exist
	sv := NewSaveVariable("v", badPath, false, false)
	err := sv.SaveToFile("x", badPath)
	if err == nil {
		t.Error("SaveToFile with nonexistent parent dir should return error")
	}
}

func TestWaitForPixel_MatchColor_invalidHexFallback(t *testing.T) {
	w := &WaitForPixel{TargetColor: "nothex", ColorTolerance: 0}
	// Both invalid hex -> fallback to exact string compare
	if !w.MatchColor("nothex") {
		t.Error("invalid hex fallback: same string should match")
	}
	if w.MatchColor("other") {
		t.Error("invalid hex fallback: different string should not match")
	}
}

func TestWaitForPixel_MatchColor_deltaBranches(t *testing.T) {
	// Hit "else" branches: sr > tr (so dr = sr - tr). Target darker than screen.
	w := &WaitForPixel{TargetColor: "000000", ColorTolerance: 5}
	if !w.MatchColor("0a0a0a") {
		t.Error("within delta when screen > target should match")
	}
	// Exact no match
	if w.MatchColor("ffffff") {
		t.Error("far color should not match")
	}
}
