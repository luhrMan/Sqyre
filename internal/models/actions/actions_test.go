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
	if got := b.String(); got != "Type: custom" {
		t.Errorf("BaseAction.String() = %q", got)
	}
	if b.Icon() == nil {
		t.Error("BaseAction.Icon() should not be nil")
	}
}

func TestAdvancedAction_String(t *testing.T) {
	adv := newAdvancedAction("myname", "loop", nil)
	if got := adv.String(); got != "Name: myname  /  Type: loop" {
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
	if c.Button != false || c.State != false {
		t.Errorf("Button=%v State=%v", c.Button, c.State)
	}
}

func TestClick_String(t *testing.T) {
	if got := NewClick(false, false).String(); got != "Type: click  /  Button: left  /  State: up" {
		t.Errorf("String() = %q", got)
	}
	if got := NewClick(true, false).String(); got != "Type: click  /  Button: right  /  State: up" {
		t.Errorf("String() = %q", got)
	}
	if got := NewClick(false, true).String(); got != "Type: click  /  Button: left  /  State: down" {
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
	if w.GetType() != "wait" {
		t.Errorf("Type=%q", w.GetType())
	}
	if time, ok := w.Time.(int); !ok || time != 500 {
		t.Errorf("Time=%v", w.Time)
	}
}

func TestNewWait_Variable(t *testing.T) {
	w := NewWait("${delay}")
	if time, ok := w.Time.(string); !ok || time != "${delay}" {
		t.Errorf("Time=%v", w.Time)
	}
}

func TestWait_String_Variable(t *testing.T) {
	if got := NewWait("${delay}").String(); got != "Type: wait  /  Time: ${delay}" {
		t.Errorf("String() = %q", got)
	}
}

func TestWait_String(t *testing.T) {
	if got := NewWait(100).String(); got != "Type: wait  /  Time: 100 ms" {
		t.Errorf("String() = %q", got)
	}
}

// --- Move ---

func TestNewMove(t *testing.T) {
	ref := NewCoordinateRef("prog", "center")
	m := NewMove(ref, false)
	if m.GetType() != "move" || m.Point != ref || m.Smooth != false {
		t.Errorf("Move: %+v", m)
	}
}

func TestNewMove_Smooth(t *testing.T) {
	m := NewMove(NewCoordinateRef("prog", "B"), true)
	if !m.Smooth {
		t.Errorf("expected Smooth=true")
	}
	if m.SmoothLow != DefaultSmoothLow || m.SmoothHigh != DefaultSmoothHigh || m.SmoothDelayMs != DefaultSmoothDelayMs {
		t.Errorf("expected default smooth settings, got low=%v high=%v delay=%v", m.SmoothLow, m.SmoothHigh, m.SmoothDelayMs)
	}
}

func TestMove_String(t *testing.T) {
	m := NewMove(NewCoordinateRef("prog", "A"), false)
	if got := m.String(); got != "Type: move  /  Point: prog~A" {
		t.Errorf("String() = %q", got)
	}
	m2 := NewMove(NewCoordinateRef("prog", "A"), true)
	if got := m2.String(); got != "Type: move  /  Point: prog~A  /  Smooth: true  /  Smooth low: 0.1  /  Smooth high: 0.5  /  Smooth delay (ms): 1" {
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
	if got := NewSetVariable("foo", "bar").String(); got != "Type: setvariable  /  Variable: foo  /  Value: bar" {
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
	if got := NewSaveVariable("x", "clipboard", false, false).String(); got != "Type: savevariable  /  Variable: x  /  Destination: clipboard  /  Mode: overwrite  /  Append Newline: off" {
		t.Errorf("String() = %q", got)
	}
	if got := NewSaveVariable("x", "/f", true, false).String(); got != "Type: savevariable  /  Variable: x  /  Destination: /f  /  Mode: append  /  Append Newline: off" {
		t.Errorf("String() = %q", got)
	}
	if got := NewSaveVariable("x", "/f", false, false).String(); got != "Type: savevariable  /  Variable: x  /  Destination: /f  /  Mode: overwrite  /  Append Newline: off" {
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
	if got := NewCalculate("x+y", "z").String(); got != "Type: calculate  /  Expression: x+y  /  Output: z" {
		t.Errorf("String() = %q", got)
	}
}

// --- FocusWindow ---

func TestNewFocusWindow(t *testing.T) {
	f := NewFocusWindow("/usr/bin/chrome", "New Tab - Chrome")
	if f.GetType() != "focuswindow" || f.ProcessPath != "/usr/bin/chrome" || f.WindowTitle != "New Tab - Chrome" {
		t.Errorf("FocusWindow: %+v", f)
	}
}

func TestFocusWindow_String(t *testing.T) {
	if got := NewFocusWindow("/usr/bin/code", "main.go - Code").String(); got != "Type: focuswindow  /  Title: main.go - Code  /  App: /usr/bin/code" {
		t.Errorf("String() = %q", got)
	}
	if got := NewFocusWindow("", "").String(); got != "Type: focuswindow  /  Title: not set  /  App: not set" {
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
	if got := l.String(); got != "Type: loop  /  Name: L  /  Iterations: 3" {
		t.Errorf("String() = %q", got)
	}
}

// --- Ocr ---

func TestNewOcr(t *testing.T) {
	area := NewCoordinateRef("prog", "box")
	o := NewOcr("myocr", "target", area)
	if o.GetType() != "ocr" || o.Name != "myocr" || o.Target != "target" || o.SearchArea.Name() != "box" {
		t.Errorf("Ocr: %+v", o)
	}
	if o.Blur != 1 || o.MinThreshold != 0 || o.Resize != 1.0 || !o.Grayscale || o.ThresholdOtsu || o.ThresholdInvert {
		t.Errorf("Ocr defaults: Blur=%d MinThreshold=%d Resize=%v Grayscale=%v ThresholdOtsu=%v ThresholdInvert=%v",
			o.Blur, o.MinThreshold, o.Resize, o.Grayscale, o.ThresholdOtsu, o.ThresholdInvert)
	}
}

func TestOcr_String(t *testing.T) {
	area := NewCoordinateRef("prog", "A")
	o := NewOcr("O", "text", area)
	got := o.String()
	if !strings.Contains(got, "O") || !strings.Contains(got, "text") || !strings.Contains(got, "Search Area: prog~A") || !strings.Contains(got, "instant") {
		t.Errorf("String() = %q", got)
	}
	if strings.Contains(got, "TopY") || strings.Contains(got, "LeftX") {
		t.Errorf("String() should not show search area coordinates, got %q", got)
	}
	o.WaitTilFound = true
	o.WaitTilFoundSeconds = 10
	if got := o.String(); !strings.Contains(got, "10 seconds") {
		t.Errorf("String() = %q", got)
	}
}

// --- ImageSearch ---

func TestNewImageSearch(t *testing.T) {
	area := NewCoordinateRef("prog", "region")
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
	area := NewCoordinateRef("prog", "R")
	is := NewImageSearch("S", nil, []string{"a"}, area, 1, 1, 0, 0)
	got := is.String()
	if !strings.Contains(got, "Items: 1") || !strings.Contains(got, "Search Area: prog~R") {
		t.Errorf("String() = %q", got)
	}
	if strings.Contains(got, "TopY") || strings.Contains(got, "LeftX") {
		t.Errorf("String() should not show search area coordinates, got %q", got)
	}
	is.WaitTilFound = true
	is.WaitTilFoundSeconds = 5
	if got := is.String(); !strings.Contains(got, "5 seconds") {
		t.Errorf("String() = %q", got)
	}
}

// --- FindPixel ---

func TestNewFindPixel(t *testing.T) {
	sa := NewCoordinateRef("prog", "sa")
	w := NewFindPixel("fp", sa, "#ff0000", 5)
	if w.GetType() != "findpixel" || w.Name != "fp" || w.ColorTolerance != 5 {
		t.Errorf("FindPixel: %+v", w)
	}
	if w.TargetColor != "ff0000" {
		t.Errorf("TargetColor = %q", w.TargetColor)
	}
}

func TestNewFindPixel_clampTolerance(t *testing.T) {
	w := NewFindPixel("w", "", "#000", -1)
	if w.ColorTolerance != 0 {
		t.Errorf("ColorTolerance should be clamped to 0, got %d", w.ColorTolerance)
	}
	w2 := NewFindPixel("w2", "", "#000", 150)
	if w2.ColorTolerance != 100 {
		t.Errorf("ColorTolerance should be clamped to 100, got %d", w2.ColorTolerance)
	}
}

func TestFindPixel_NormalizeHex(t *testing.T) {
	w := &FindPixel{}
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

func TestFindPixel_MatchColor(t *testing.T) {
	w := &FindPixel{TargetColor: "ffffff", ColorTolerance: 0}
	if !w.MatchColor("#ffffff") {
		t.Error("exact match should be true")
	}
	if w.MatchColor("#000000") {
		t.Error("different color with 0 tolerance should be false")
	}
	w.ColorTolerance = 100
	if !w.MatchColor("#000000") {
		t.Error("100% tolerance should match any valid hex")
	}
	w.ColorTolerance = 50
	w.TargetColor = "808080"
	if !w.MatchColor("c0c0c0") {
		t.Error("within tolerance should match")
	}
}

func TestFindPixel_String(t *testing.T) {
	sa := NewCoordinateRef("prog", "region")
	w := NewFindPixel("W", sa, "ff0000", 0)
	got := w.String()
	if !strings.Contains(got, "instant") || !strings.Contains(got, "ff0000") || !strings.Contains(got, "Search Area: prog~region") {
		t.Errorf("String() = %q", got)
	}
	if strings.Contains(got, "TopY") || strings.Contains(got, "LeftX") {
		t.Errorf("String() should not show search area coordinates, got %q", got)
	}
	w.WaitTilFound = true
	w.WaitTilFoundSeconds = 5
	if got := w.String(); !strings.Contains(got, "5") {
		t.Errorf("String() = %q", got)
	}
}

func TestForEachRow_SourcesAndSubActions(t *testing.T) {
	sub := NewWait(5)
	fer := NewForEachRow("rows", []ListColumn{
		{Source: "a", OutputVar: "x"},
		{Source: "b", OutputVar: "y"},
	}, []ActionInterface{sub})
	if fer.Name != "rows" || len(fer.Sources) != 2 {
		t.Errorf("ForEachRow: %+v", fer)
	}
	subs := fer.GetSubActions()
	if len(subs) != 1 || subs[0] != sub {
		t.Errorf("GetSubActions() = %+v", subs)
	}
}

// TestActionTypes_Icon ensures Icon() is called on every action type for coverage.
func TestActionTypes_Icon(t *testing.T) {
	actions := []struct {
		name string
		a    ActionInterface
	}{
		{"Calculate", NewCalculate("1", "x")},
		{"ForEachRow", NewForEachRow("r", nil, nil)},
		{"FocusWindow", NewFocusWindow("/app", "Window")},
		{"ImageSearch", NewImageSearch("s", nil, nil, "", 0, 0, 0, 0)},
		{"Key", NewKey("k", false)},
		{"KeyDown", NewKey("k", true)},
		{"Loop", NewLoop(1, "l", nil)},
		{"Move", NewMove("", false)},
		{"Ocr", NewOcr("o", "", "")},
		{"SaveVariable", NewSaveVariable("v", "d", false, false)},
		{"SetVariable", NewSetVariable("v", 0)},
		{"Wait", NewWait(0)},
		{"FindPixel", NewFindPixel("w", "", "000000", 0)},
		{"Break", NewBreak()},
		{"Continue", NewContinue()},
	}
	for _, tt := range actions {
		t.Run(tt.name, func(t *testing.T) {
			if tt.a.Icon() == nil {
				t.Error("Icon() should not be nil")
			}
		})
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

func TestFindPixel_MatchColor_invalidHexFallback(t *testing.T) {
	w := &FindPixel{TargetColor: "nothex", ColorTolerance: 0}
	if !w.MatchColor("nothex") {
		t.Error("invalid hex fallback: same string should match")
	}
	if w.MatchColor("other") {
		t.Error("invalid hex fallback: different string should not match")
	}
}

func TestFindPixel_MatchColor_deltaBranches(t *testing.T) {
	w := &FindPixel{TargetColor: "000000", ColorTolerance: 5}
	if !w.MatchColor("0a0a0a") {
		t.Error("within delta when screen > target should match")
	}
	if w.MatchColor("ffffff") {
		t.Error("far color should not match")
	}
}
