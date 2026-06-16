package services

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func withRecordingBackend(t *testing.T) *RecordingBackend {
	t.Helper()
	rec := &RecordingBackend{}
	SetAutomationBackend(rec)
	t.Cleanup(ResetAutomationBackend)
	return rec
}

func TestExecute_Wait(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewWait(250), nil); err != nil {
		t.Fatalf("Execute wait: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Op != "sleep" || rec.Calls[0].Ms != 250 {
		t.Fatalf("calls = %+v, want single sleep 250ms", rec.Calls)
	}
}

func TestExecute_Move(t *testing.T) {
	rec := withRecordingBackend(t)
	move := actions.NewMove(actions.Point{Name: "p", X: 10, Y: 20}, true)
	if err := Execute(move, nil); err != nil {
		t.Fatalf("Execute move: %v", err)
	}
	if len(rec.Calls) != 1 {
		t.Fatalf("calls = %+v", rec.Calls)
	}
	c := rec.Calls[0]
	if c.Op != "move" || c.X != 10 || c.Y != 20 || !c.Smooth {
		t.Fatalf("move call = %+v", c)
	}
}

func TestExecute_Click(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewClick(true, false), nil); err != nil {
		t.Fatalf("Execute click: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Op != "click" || rec.Calls[0].Button != "right" || rec.Calls[0].Down {
		t.Fatalf("calls = %+v", rec.Calls)
	}
}

func TestExecute_Key(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewKey("a", true), nil); err != nil {
		t.Fatalf("Execute key down: %v", err)
	}
	if err := Execute(actions.NewKey("a", false), nil); err != nil {
		t.Fatalf("Execute key up: %v", err)
	}
	if len(rec.Calls) != 2 || rec.Calls[0].Op != "keydown" || rec.Calls[1].Op != "keyup" {
		t.Fatalf("calls = %+v", rec.Calls)
	}
}

func TestExecute_Type(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewType("ab", 0), nil); err != nil {
		t.Fatalf("Execute type: %v", err)
	}
	if len(rec.Calls) != 2 || rec.Calls[0].Char != "a" || rec.Calls[1].Char != "b" {
		t.Fatalf("calls = %+v", rec.Calls)
	}
}

func TestExecute_SetVariable(t *testing.T) {
	withRecordingBackend(t)
	macro := models.NewMacro("test", 0, nil)
	sv := actions.NewSetVariable("count", 42)
	if err := Execute(sv, macro); err != nil {
		t.Fatalf("Execute set variable: %v", err)
	}
	val, ok := macro.Variables.Get("count")
	if !ok || val != 42 {
		t.Fatalf("count = %v ok=%v", val, ok)
	}
}

func TestExecute_LoopRunsSubActions(t *testing.T) {
	rec := withRecordingBackend(t)
	loop := actions.NewLoop(2, "inner", []actions.ActionInterface{
		actions.NewWait(10),
		actions.NewWait(20),
	})
	if err := Execute(loop, nil); err != nil {
		t.Fatalf("Execute loop: %v", err)
	}
	if len(rec.Calls) != 4 {
		t.Fatalf("expected 4 sleep calls, got %d: %+v", len(rec.Calls), rec.Calls)
	}
}

func TestExecute_SaveVariableClipboard(t *testing.T) {
	rec := withRecordingBackend(t)
	macro := models.NewMacro("test", 0, nil)
	macro.Variables.Set("msg", "hello")
	sv := actions.NewSaveVariable("msg", "clipboard", false, false)
	if err := Execute(sv, macro); err != nil {
		t.Fatalf("Execute save variable: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Op != "clipboard" || rec.Calls[0].Text != "hello" {
		t.Fatalf("calls = %+v", rec.Calls)
	}
}
