package services

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func TestExecute_ForEachRowMultipleSources(t *testing.T) {
	rec := withRecordingBackend(t)
	macro := models.NewMacro("test", 0, nil)
	fer := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "a\nb", OutputVar: "letter"},
		{Source: "1\n2", OutputVar: "digit"},
	}, []actions.ActionInterface{
		actions.NewWait(10),
	})
	if err := Execute(fer, macro); err != nil {
		t.Fatalf("Execute for each row: %v", err)
	}
	if len(rec.Calls) != 2 {
		t.Fatalf("expected 2 waits, got %d", len(rec.Calls))
	}
	letter, _ := macro.Variables.Get("letter")
	if letter != "b" {
		t.Fatalf("letter = %v, want b", letter)
	}
	digit, _ := macro.Variables.Get("digit")
	if digit != "2" {
		t.Fatalf("digit = %v, want 2", digit)
	}
	row, _ := macro.Variables.Get(actions.ForEachRowBuiltinRow)
	if row != 2 {
		t.Fatalf("Row = %v, want 2", row)
	}
	count, _ := macro.Variables.Get(actions.ForEachRowBuiltinRowCount)
	if count != 2 {
		t.Fatalf("RowCount = %v, want 2", count)
	}
}
