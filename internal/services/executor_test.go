package services

import (
	"os"
	"path/filepath"
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/models/serialize"
)

func withRecordingBackend(t *testing.T) *RecordingBackend {
	t.Helper()
	rec := &RecordingBackend{}
	SetAutomationBackend(rec)
	t.Cleanup(ResetAutomationBackend)
	return rec
}

func totalSleepMs(calls []RecordedCall) int {
	total := 0
	for _, c := range calls {
		if c.Op == "sleep" {
			total += c.Ms
		}
	}
	return total
}

func TestExecute_Wait(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewWait(250), nil); err != nil {
		t.Fatalf("Execute wait: %v", err)
	}
	if totalSleepMs(rec.Calls) != 250 {
		t.Fatalf("calls = %+v, want 250ms total sleep", rec.Calls)
	}
}

func TestExecute_Move(t *testing.T) {
	rec := withRecordingBackend(t)
	initTestConfig(t)
	program := repositories.ProgramRepo().New()
	program.Name = "test-program"
	if err := program.PointRepo(DefaultResolutionKey()).Set("p", &models.Point{Name: "p", X: 10, Y: 20}); err != nil {
		t.Fatalf("set point: %v", err)
	}
	if err := repositories.ProgramRepo().Set("test-program", program); err != nil {
		t.Fatalf("set program: %v", err)
	}
	move := actions.NewMove(actions.NewCoordinateRef("test-program", "p"), true)
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
	if c.SmoothLow != actions.DefaultSmoothLow || c.SmoothHigh != actions.DefaultSmoothHigh || c.SmoothDelayMs != actions.DefaultSmoothDelayMs {
		t.Fatalf("smooth settings = low=%v high=%v delay=%v", c.SmoothLow, c.SmoothHigh, c.SmoothDelayMs)
	}
}

func initTestConfig(t *testing.T) {
	t.Helper()
	os.Setenv("SQYRE_TEST_MODE", "1")
	dir := t.TempDir()
	configPath := filepath.Join(dir, "db.yaml")
	if err := os.WriteFile(configPath, []byte("macros: {}\nprograms: {}\n"), 0644); err != nil {
		t.Fatalf("write temp config: %v", err)
	}
	repositories.ResetAllForTesting()
	yamlConfig := serialize.GetYAMLConfig()
	yamlConfig.SetConfigFile(configPath)
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("read temp config: %v", err)
	}
	viperCfg := serialize.GetViper()
	viperCfg.SetConfigFile(configPath)
	viperCfg.SetConfigType("yaml")
	if err := viperCfg.ReadInConfig(); err != nil {
		t.Fatalf("read viper config: %v", err)
	}
}

func TestExecute_Click(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewClick(actions.ClickButtonRight, true), nil); err != nil {
		t.Fatalf("Execute click: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Op != "click" || rec.Calls[0].Button != "right" || !rec.Calls[0].Down {
		t.Fatalf("calls = %+v", rec.Calls)
	}
}

func TestExecute_ClickCenter(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewClick(actions.ClickButtonCenter, false), nil); err != nil {
		t.Fatalf("Execute center click: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Op != "click" || rec.Calls[0].Button != "center" || rec.Calls[0].Down {
		t.Fatalf("calls = %+v", rec.Calls)
	}
}

func TestExecute_ClickScroll(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewClick(actions.ClickButtonScroll, false), nil); err != nil {
		t.Fatalf("Execute scroll: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Op != "scroll" || rec.Calls[0].Button != "up" {
		t.Fatalf("calls = %+v", rec.Calls)
	}
	if err := Execute(actions.NewClick(actions.ClickButtonScroll, true), nil); err != nil {
		t.Fatalf("Execute scroll down: %v", err)
	}
	if len(rec.Calls) != 2 || rec.Calls[1].Op != "scroll" || rec.Calls[1].Button != "down" {
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

func TestExecute_LoopBreak(t *testing.T) {
	rec := withRecordingBackend(t)
	loop := actions.NewLoop(5, "inner", []actions.ActionInterface{
		actions.NewWait(10),
		actions.NewBreak(),
		actions.NewWait(20),
	})
	if err := Execute(loop, nil); err != nil {
		t.Fatalf("Execute loop break: %v", err)
	}
	if len(rec.Calls) != 1 || rec.Calls[0].Ms != 10 {
		t.Fatalf("expected single 10ms sleep before break, got %+v", rec.Calls)
	}
}

func TestExecute_LoopContinue(t *testing.T) {
	rec := withRecordingBackend(t)
	loop := actions.NewLoop(2, "inner", []actions.ActionInterface{
		actions.NewWait(10),
		actions.NewContinue(),
		actions.NewWait(20),
	})
	if err := Execute(loop, nil); err != nil {
		t.Fatalf("Execute loop continue: %v", err)
	}
	if len(rec.Calls) != 2 {
		t.Fatalf("expected 2 sleep calls, got %d: %+v", len(rec.Calls), rec.Calls)
	}
	for _, c := range rec.Calls {
		if c.Ms != 10 {
			t.Fatalf("expected only 10ms sleeps, got %+v", rec.Calls)
		}
	}
}

func TestExecute_LoopBreakViaConditional(t *testing.T) {
	rec := withRecordingBackend(t)
	loop := actions.NewLoop(3, "inner", []actions.ActionInterface{
		actions.NewConditional([]actions.ConditionClause{
			{Left: 1, Operator: actions.OpEquals, Right: 1},
		}, actions.MatchAll, "c", []actions.ActionInterface{
			actions.NewBreak(),
		}),
		actions.NewWait(10),
	})
	if err := Execute(loop, nil); err != nil {
		t.Fatalf("Execute loop break via conditional: %v", err)
	}
	if len(rec.Calls) != 0 {
		t.Fatalf("expected no waits after break on first iteration, got %+v", rec.Calls)
	}
}

func TestExecute_ForEachRowContinue(t *testing.T) {
	rec := withRecordingBackend(t)
	macro := models.NewMacro("test", 0, nil)
	fer := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "a\nb", OutputVar: "letter"},
	}, []actions.ActionInterface{
		actions.NewWait(10),
		actions.NewContinue(),
		actions.NewWait(20),
	})
	if err := Execute(fer, macro); err != nil {
		t.Fatalf("Execute for each row continue: %v", err)
	}
	if len(rec.Calls) != 2 {
		t.Fatalf("expected 2 waits, got %d: %+v", len(rec.Calls), rec.Calls)
	}
}

func TestExecute_NestedLoopBreak(t *testing.T) {
	rec := withRecordingBackend(t)
	outer := actions.NewLoop(3, "outer", []actions.ActionInterface{
		actions.NewLoop(3, "inner", []actions.ActionInterface{
			actions.NewWait(10),
			actions.NewBreak(),
			actions.NewWait(20),
		}),
		actions.NewWait(30),
	})
	if err := Execute(outer, nil); err != nil {
		t.Fatalf("Execute nested loop break: %v", err)
	}
	count10, count30 := 0, 0
	for _, c := range rec.Calls {
		switch c.Ms {
		case 10:
			count10++
		case 30:
			count30++
		}
	}
	// Innermost-only break: outer keeps iterating (3x inner wait + 3x outer wait after inner).
	if count10 != 3 || count30 != 3 {
		t.Fatalf("expected 3x10ms and 3x30ms (innermost break only), got 10ms=%d 30ms=%d calls=%+v", count10, count30, rec.Calls)
	}
}

func TestExecute_BreakOutsideLoopIgnored(t *testing.T) {
	rec := withRecordingBackend(t)
	if err := Execute(actions.NewBreak(), nil); err != nil {
		t.Fatalf("standalone break should not error: %v", err)
	}
	if len(rec.Calls) != 0 {
		t.Fatalf("unexpected calls: %+v", rec.Calls)
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

func TestExecute_GlobalDelayBetweenActions(t *testing.T) {
	rec := withRecordingBackend(t)
	macro := models.NewMacro("test", 50, nil)
	loop := actions.NewLoop(1, "inner", []actions.ActionInterface{
		actions.NewWait(10),
		actions.NewWait(20),
	})
	if err := Execute(loop, macro); err != nil {
		t.Fatalf("Execute loop with global delay: %v", err)
	}
	want := []RecordedCall{
		{Op: "sleep", Ms: 10},
		{Op: "sleep", Ms: 50},
		{Op: "sleep", Ms: 20},
		{Op: "sleep", Ms: 50},
		{Op: "sleep", Ms: 50}, // after loop container completes
	}
	if len(rec.Calls) != len(want) {
		t.Fatalf("calls = %+v, want %d entries", rec.Calls, len(want))
	}
	for i, c := range rec.Calls {
		if c.Op != want[i].Op || c.Ms != want[i].Ms {
			t.Fatalf("call[%d] = %+v, want %+v", i, c, want[i])
		}
	}
}

func TestExecute_GlobalDelaySkippedWhenZero(t *testing.T) {
	rec := withRecordingBackend(t)
	macro := models.NewMacro("test", 0, nil)
	loop := actions.NewLoop(1, "inner", []actions.ActionInterface{
		actions.NewWait(10),
		actions.NewWait(20),
	})
	if err := Execute(loop, macro); err != nil {
		t.Fatalf("Execute loop: %v", err)
	}
	if len(rec.Calls) != 2 {
		t.Fatalf("expected 2 sleep calls without global delay, got %d: %+v", len(rec.Calls), rec.Calls)
	}
}

func TestExecute_GlobalDelayAfterNonInputAction(t *testing.T) {
	rec := withRecordingBackend(t)
	macro := models.NewMacro("test", 75, nil)
	sv := actions.NewSetVariable("count", 1)
	if err := Execute(sv, macro); err != nil {
		t.Fatalf("Execute set variable: %v", err)
	}
	if totalSleepMs(rec.Calls) != 75 {
		t.Fatalf("calls = %+v, want 75ms global delay after set variable", rec.Calls)
	}
}
