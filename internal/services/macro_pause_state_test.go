package services

import (
	"testing"
	"time"
)

func TestShouldEscapeStopMacro(t *testing.T) {
	if ShouldEscapeStopMacro() {
		t.Fatal("escape should not stop when no macro is running")
	}

	if !tryStartMacroRun("test") {
		t.Fatal("start should succeed")
	}
	defer endMacroRun()

	if !ShouldEscapeStopMacro() {
		t.Fatal("escape should stop while macro is running")
	}

	BeginMacroPauseWait(false)
	if !ShouldEscapeStopMacro() {
		t.Fatal("escape should stop during pause when continue key is not escape")
	}
	EndMacroPauseWait()

	BeginMacroPauseWait(true)
	if ShouldEscapeStopMacro() {
		t.Fatal("escape should resume pause when continue key is escape")
	}
	EndMacroPauseWait()

	if !ShouldEscapeStopMacro() {
		t.Fatal("escape should stop again after pause ends")
	}

	BeginMacroKeyActionEscape()
	if ShouldEscapeStopMacro() {
		t.Fatal("escape should not stop while a Key action is sending escape")
	}
	EndMacroKeyActionEscape()

	if !ShouldEscapeStopMacro() {
		t.Fatal("escape should stop again after Key action ends")
	}
}

func TestMacroEscapeSuppressGrace(t *testing.T) {
	if !tryStartMacroRun("test") {
		t.Fatal("start should succeed")
	}
	defer endMacroRun()

	extendMacroEscapeSuppress(80 * time.Millisecond)
	if ShouldEscapeStopMacro() {
		t.Fatal("escape should be suppressed during grace window")
	}
	time.Sleep(100 * time.Millisecond)
	if !ShouldEscapeStopMacro() {
		t.Fatal("escape should stop again after grace window")
	}
}

func TestIsEscapeKey(t *testing.T) {
	for _, key := range []string{"esc", "ESC", " escape ", "Escape"} {
		if !IsEscapeKey(key) {
			t.Fatalf("IsEscapeKey(%q) = false, want true", key)
		}
	}
	if IsEscapeKey("enter") {
		t.Fatal("IsEscapeKey(enter) = true, want false")
	}
}
