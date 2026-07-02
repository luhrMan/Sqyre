package completionentry

import (
	"testing"
	"time"
)

func TestIsCompletionActive_tracksShowHide(t *testing.T) {
	if IsCompletionActive() {
		t.Fatal("expected no active completion initially")
	}
	registerCompletionShown()
	if !IsCompletionActive() {
		t.Fatal("expected active completion after show")
	}
	registerCompletionHidden()
	if IsCompletionActive() {
		t.Fatal("expected no active completion after hide")
	}
}

func TestIsActionDialogEnterSuppressed(t *testing.T) {
	suppressActionDialogEnter()
	if !IsActionDialogEnterSuppressed() {
		t.Fatal("expected Enter to be suppressed immediately after completion handled it")
	}
	time.Sleep(enterSuppressDuration + 10*time.Millisecond)
	if IsActionDialogEnterSuppressed() {
		t.Fatal("expected Enter suppression to expire")
	}
}

func TestRegisterCompletionHidden_neverNegative(t *testing.T) {
	registerCompletionHidden()
	registerCompletionHidden()
	if activeCompletions.Load() != 0 {
		t.Fatalf("active completions = %d, want 0", activeCompletions.Load())
	}
}
