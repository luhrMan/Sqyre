package actions

import "testing"

func TestConditionClause_Summary(t *testing.T) {
	binary := ConditionClause{Left: "${score}", Operator: OpLess, Right: 10}
	if got := binary.Summary(); got != "${score} < 10" {
		t.Fatalf("Summary() = %q, want %q", got, "${score} < 10")
	}
	unary := ConditionClause{Left: "${name}", Operator: OpIsSet}
	if got := unary.Summary(); got != "${name} is set" {
		t.Fatalf("Summary() = %q, want %q", got, "${name} is set")
	}
}

func TestConditional_conditionSummary(t *testing.T) {
	and := NewConditional([]ConditionClause{
		{Left: "a", Operator: OpEquals, Right: "1"},
		{Left: "b", Operator: OpEquals, Right: "2"},
	}, MatchAll, "x", nil)
	if got := and.conditionSummary(); got != "a == 1 AND b == 2" {
		t.Fatalf("AND summary = %q", got)
	}
	or := NewConditional([]ConditionClause{
		{Left: "a", Operator: OpEquals, Right: "1"},
		{Left: "b", Operator: OpEquals, Right: "2"},
	}, MatchAny, "x", nil)
	if got := or.conditionSummary(); got != "a == 1 OR b == 2" {
		t.Fatalf("OR summary = %q", got)
	}
}

func TestNewConditional_defaultClause(t *testing.T) {
	c := NewConditional(nil, "", "test", nil)
	if len(c.Clauses) != 1 {
		t.Fatalf("expected 1 default clause, got %d", len(c.Clauses))
	}
	if c.Clauses[0].Operator != OpEquals {
		t.Fatalf("default operator = %q", c.Clauses[0].Operator)
	}
}

func TestConditional_EffectiveMatch(t *testing.T) {
	c := NewConditional(nil, MatchAny, "", nil)
	if c.EffectiveMatch() != MatchAny {
		t.Fatalf("EffectiveMatch() = %q, want %q", c.EffectiveMatch(), MatchAny)
	}
	c.Match = ""
	if c.EffectiveMatch() != MatchAll {
		t.Fatalf("empty match should default to %q", MatchAll)
	}
}
