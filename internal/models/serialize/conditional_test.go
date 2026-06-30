package serialize

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestActionToMap_conditionalClausesRoundTrip(t *testing.T) {
	orig := actions.NewConditional([]actions.ConditionClause{
		{Left: "${a}", Operator: actions.OpEquals, Right: 1},
		{Left: "${b}", Operator: actions.OpContains, Right: "ok"},
	}, actions.MatchAny, "if", []actions.ActionInterface{
		actions.NewBreak(),
	})

	m, err := ActionToMap(orig)
	if err != nil {
		t.Fatal(err)
	}

	back, err := ViperSerializer.CreateActionFromMap(m, nil)
	if err != nil {
		t.Fatal(err)
	}
	cond, ok := back.(*actions.Conditional)
	if !ok {
		t.Fatalf("expected *Conditional, got %T", back)
	}
	if cond.EffectiveMatch() != actions.MatchAny {
		t.Fatalf("match = %q", cond.EffectiveMatch())
	}
	if len(cond.Clauses) != 2 {
		t.Fatalf("clauses = %d", len(cond.Clauses))
	}
	if cond.Clauses[1].Operator != actions.OpContains {
		t.Fatalf("clause 2 operator = %q", cond.Clauses[1].Operator)
	}
	if len(cond.GetSubActions()) != 1 {
		t.Fatalf("subactions = %d", len(cond.GetSubActions()))
	}
}

func TestCreateActionFromMap_conditionalMissingClausesDefaults(t *testing.T) {
	back, err := ViperSerializer.CreateActionFromMap(map[string]any{
		"type": "conditional",
		"name": "if",
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	cond := back.(*actions.Conditional)
	if len(cond.Clauses) != 1 || cond.Clauses[0].Operator != actions.OpEquals {
		t.Fatalf("clauses = %+v", cond.Clauses)
	}
}
