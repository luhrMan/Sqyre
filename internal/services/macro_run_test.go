package services

import (
	"testing"
	"time"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func TestMacroRun_OnlyOneAtATime(t *testing.T) {
	if !tryStartMacroRun("first") {
		t.Fatal("first start should succeed")
	}
	if tryStartMacroRun("second") {
		t.Fatal("second start should fail while first is running")
	}
	if RunningMacroName() != "first" {
		t.Fatalf("RunningMacroName() = %q, want first", RunningMacroName())
	}
	endMacroRun()
	if RunningMacroName() != "" {
		t.Fatalf("RunningMacroName() = %q after end, want empty", RunningMacroName())
	}
	if !tryStartMacroRun("second") {
		t.Fatal("start after end should succeed")
	}
	endMacroRun()
}

func TestMacroRun_StopRequest(t *testing.T) {
	if !tryStartMacroRun("test") {
		t.Fatal("start should succeed")
	}
	defer endMacroRun()

	if MacroStopPending() {
		t.Fatal("stop should not be pending at start")
	}
	RequestMacroStop()
	if !MacroStopPending() {
		t.Fatal("stop should be pending after RequestMacroStop")
	}
	endMacroRun()
	if MacroStopPending() {
		t.Fatal("stop flag should clear when run ends")
	}
}

func TestExecute_StopDuringWait(t *testing.T) {
	rec := withRecordingBackend(t)
	rec.RealSleep = true
	if !tryStartMacroRun("test") {
		t.Fatal("start should succeed")
	}
	defer endMacroRun()

	done := make(chan error, 1)
	go func() {
		done <- Execute(actions.NewWait(5000), nil)
	}()

	time.Sleep(120 * time.Millisecond)
	RequestMacroStop()

	select {
	case err := <-done:
		if !actions.IsStopped(err) {
			t.Fatalf("Execute wait: got %v, want ErrStopped", err)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("Execute wait did not stop in time")
	}

	totalSleep := 0
	for _, c := range rec.Calls {
		if c.Op == "sleep" {
			totalSleep += c.Ms
		}
	}
	if totalSleep >= 5000 {
		t.Fatalf("slept %d ms, expected early stop before 5000 ms", totalSleep)
	}
}

func TestExecuteMacroWithLogging_RejectsConcurrentRun(t *testing.T) {
	if !tryStartMacroRun("busy") {
		t.Fatal("start should succeed")
	}
	defer endMacroRun()

	ExecuteMacroWithLogging(models.NewMacro("other", 0, nil))
	if RunningMacroName() != "busy" {
		t.Fatalf("running macro = %q, want busy", RunningMacroName())
	}
}
