package actions_test

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestDisplayParams_splitsSummaryAndExtra(t *testing.T) {
	params := []actions.Param{
		{Label: "Type", Value: "move"},
		{Label: "Point", Value: "home"},
		{Label: "Smooth", Value: true, Extra: true},
		{Label: "Smooth low", Value: 0.05, Extra: true},
	}
	summary, extra := actions.DisplayParams(params)
	if len(summary) != 1 || summary[0].Label != "Point" {
		t.Fatalf("summary = %#v, want Point only", summary)
	}
	if len(extra) != 2 {
		t.Fatalf("extra = %#v, want 2 params", extra)
	}
}

func TestFormatParamMinimal_valueOnly(t *testing.T) {
	p := actions.Param{Label: "Point", Value: "home"}
	if got := actions.FormatParamMinimal(p); got != "home" {
		t.Fatalf("FormatParamMinimal() = %q, want home", got)
	}
}

func TestMoveParams_smoothMarkedExtra(t *testing.T) {
	m := actions.NewMove("pt", true)
	_, extra := actions.DisplayParams(m.Params())
	if len(extra) != 4 {
		t.Fatalf("smooth move extra params = %d, want 4", len(extra))
	}
}

func TestClickParams_noExtra(t *testing.T) {
	c := actions.NewClick(actions.ClickButtonLeft, true)
	_, extra := actions.DisplayParams(c.Params())
	if len(extra) != 0 {
		t.Fatalf("click extra params = %d, want 0", len(extra))
	}
}
