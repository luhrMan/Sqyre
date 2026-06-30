package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func TestResolveIntWithOverrides_ImagePixelWidth(t *testing.T) {
	overrides := map[string]any{
		"ImagePixelWidth":  64,
		"ImagePixelHeight": 32,
	}
	n, err := ResolveIntWithOverrides("${ImagePixelWidth}", nil, overrides)
	if err != nil {
		t.Fatal(err)
	}
	if n != 64 {
		t.Fatalf("got %d want 64", n)
	}

	n, err = ResolveIntWithOverrides("${ImagePixelWidth}/2", nil, overrides)
	if err != nil {
		t.Fatal(err)
	}
	if n != 32 {
		t.Fatalf("got %d want 32", n)
	}
}

func TestResolveSetVariableValue(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.Variables.Set("x", 10)

	v, err := ResolveSetVariableValue("${x}", m)
	if err != nil {
		t.Fatal(err)
	}
	if v != 10 {
		t.Fatalf("got %v want 10", v)
	}

	v, err = ResolveSetVariableValue("plain", m)
	if err != nil || v != "plain" {
		t.Fatalf("got %v err %v", v, err)
	}

	v, err = ResolveSetVariableValue(42, m)
	if err != nil || v != 42 {
		t.Fatalf("got %v err %v", v, err)
	}

	v, err = ResolveSetVariableValue("1+${x}", m)
	if err != nil {
		t.Fatal(err)
	}
	if f, ok := v.(float64); !ok || f != 11 {
		t.Fatalf("got %v (%T) want 11", v, v)
	}

	v, err = ResolveSetVariableValue("hello-world", m)
	if err != nil || v != "hello-world" {
		t.Fatalf("got %v err %v want plain string", v, err)
	}
}

func TestPreviewCalculate(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.UpsertVariable(models.VariableDecl{Name: "count", Type: models.VariableTypeNumber, InitialValue: "5"})
	m.UpsertVariable(models.VariableDecl{Name: "label", Type: models.VariableTypeText}) // no initial value
	// An action-produced variable (set at runtime, no initial value).
	calc := actions.NewCalculate("0", "result")
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{calc})

	cases := []struct {
		name    string
		expr    string
		want    string
		wantErr bool
	}{
		{"empty", "", "", false},
		{"pure arithmetic", "2 + 3", "= 5", false},
		{"declared with value", "${count} * 2", "= 10", false},
		{"declared without value", "${label} + 1", "valid (result depends on runtime values)", false},
		{"runtime output var", "${result} + 1", "valid (result depends on runtime values)", false},
		{"unknown variable", "${missing} + 1", "valid (result depends on runtime values)", false},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, err := PreviewCalculate(tc.expr, m)
			if tc.wantErr {
				if err == nil {
					t.Fatalf("expected error, got %q", got)
				}
				return
			}
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if got != tc.want {
				t.Fatalf("PreviewCalculate(%q) = %q, want %q", tc.expr, got, tc.want)
			}
		})
	}
}

func TestValidateSetVariableValue(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.UpsertVariable(models.VariableDecl{Name: "x", Type: models.VariableTypeNumber, InitialValue: "5"})

	if v := ValidateSetVariableValue("hello", m); v.BlocksSubmit() {
		t.Fatalf("plain text should be submittable: %+v", v)
	}
	if v := ValidateSetVariableValue("1+${x}", m); v.BlocksSubmit() {
		t.Fatalf("expression should be valid: %+v", v)
	}
	if v := ValidateSetVariableValue("${missing}", m); v.BlocksSubmit() {
		t.Fatal("unknown variable should warn only")
	}
	if v := ValidateSetVariableValue("${missing}", m); v.Warning == "" {
		t.Fatal("expected unknown variable warning")
	}
	if v := ValidateSetVariableValue("1 + ", m); !v.BlocksSubmit() {
		t.Fatal("expected invalid expression error")
	}
}

func TestValidateNumericExpression(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.UpsertVariable(models.VariableDecl{Name: "count", Type: models.VariableTypeNumber, InitialValue: "5"})

	if v := ValidateNumericExpression("10", m); v.BlocksSubmit() {
		t.Fatalf("literal int: %+v", v)
	}
	if v := ValidateNumericExpression("${count} * 2", m); v.BlocksSubmit() {
		t.Fatalf("expression: %+v", v)
	}
	if v := ValidateNumericExpression("${missing}", m); v.BlocksSubmit() {
		t.Fatal("unknown variable should warn only")
	}
	if v := ValidateNumericExpression("not-a-number", m); !v.BlocksSubmit() {
		t.Fatal("expected error for non-numeric text")
	}
}

func TestLooksLikeExpression(t *testing.T) {
	if !LooksLikeExpression("1+2") {
		t.Fatal("expected arithmetic to look like expression")
	}
	if LooksLikeExpression("hello") {
		t.Fatal("plain text should not look like expression")
	}
	if !LooksLikeExpression("sqrt(4)") {
		t.Fatal("function call should look like expression")
	}
}

func TestEvaluateCondition_matchAll(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.Variables.Set("a", 1)
	m.Variables.Set("b", 2)

	ok, err := EvaluateCondition(actions.NewConditional([]actions.ConditionClause{
		{Left: "${a}", Operator: actions.OpEquals, Right: 1},
		{Left: "${b}", Operator: actions.OpEquals, Right: 2},
	}, actions.MatchAll, "", nil), m)
	if err != nil || !ok {
		t.Fatalf("all true: ok=%v err=%v", ok, err)
	}

	ok, err = EvaluateCondition(actions.NewConditional([]actions.ConditionClause{
		{Left: "${a}", Operator: actions.OpEquals, Right: 1},
		{Left: "${b}", Operator: actions.OpEquals, Right: 3},
	}, actions.MatchAll, "", nil), m)
	if err != nil || ok {
		t.Fatalf("one false: ok=%v err=%v", ok, err)
	}
}

func TestEvaluateCondition_matchAny(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.Variables.Set("a", 1)
	m.Variables.Set("b", 2)

	ok, err := EvaluateCondition(actions.NewConditional([]actions.ConditionClause{
		{Left: "${a}", Operator: actions.OpEquals, Right: 9},
		{Left: "${b}", Operator: actions.OpEquals, Right: 2},
	}, actions.MatchAny, "", nil), m)
	if err != nil || !ok {
		t.Fatalf("one true: ok=%v err=%v", ok, err)
	}

	ok, err = EvaluateCondition(actions.NewConditional([]actions.ConditionClause{
		{Left: "${a}", Operator: actions.OpEquals, Right: 9},
		{Left: "${b}", Operator: actions.OpEquals, Right: 8},
	}, actions.MatchAny, "", nil), m)
	if err != nil || ok {
		t.Fatalf("none true: ok=%v err=%v", ok, err)
	}
}

func TestEvaluateCondition_emptyClauses(t *testing.T) {
	ok, err := EvaluateCondition(&actions.Conditional{Clauses: nil}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if ok {
		t.Fatal("empty clauses should be false")
	}
}

func TestEvaluateExpression_negativeVariableSubtraction(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.Variables.Set("topExpander", -30)

	result, err := EvaluateExpression("${topExpander}-250", m)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if f, ok := result.(float64); !ok || f != -280 {
		t.Fatalf("got %v (%T) want -280", result, result)
	}
}

func TestEvaluateExpression_negativeLiteral(t *testing.T) {
	m := models.NewMacro("t", 0, nil)

	result, err := EvaluateExpression("-30", m)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if f, ok := result.(float64); !ok || f != -30 {
		t.Fatalf("got %v (%T) want -30", result, result)
	}
}

func TestEvaluateExpression_negativeVariableAddition(t *testing.T) {
	m := models.NewMacro("t", 0, nil)
	m.Variables.Set("x", -5)

	result, err := EvaluateExpression("${x}+3", m)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if f, ok := result.(float64); !ok || f != -2 {
		t.Fatalf("got %v (%T) want -2", result, result)
	}
}
