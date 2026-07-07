package macro

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestAppendConditionalTooltipView_matchesEditSectionCount(t *testing.T) {
	t.Helper()
	a := actions.NewConditional([]actions.ConditionClause{
		{Left: "a", Operator: actions.OpEquals, Right: "b"},
		{Left: "c", Operator: actions.OpGreater, Right: 1},
	}, actions.MatchAll, "cond", nil)

	sections := appendConditionalTooltipView(a, a.GetType())
	if len(sections) != 3 {
		t.Fatalf("expected general + 2 clause sections, got %d", len(sections))
	}
}
