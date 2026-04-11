package services

import (
	"Sqyre/internal/models"
	"math"
	"testing"
)

func newTestMacro(vars map[string]any) *models.Macro {
	m := &models.Macro{Variables: models.NewVariableStore()}
	for k, v := range vars {
		m.Variables.Set(k, v)
	}
	return m
}

func TestResolveVariables_DollarBrace(t *testing.T) {
	m := newTestMacro(map[string]any{"x": 42, "name": "hello"})
	got, err := ResolveVariables("val=${x} name=${name}", m)
	if err != nil {
		t.Fatal(err)
	}
	if got != "val=42 name=hello" {
		t.Errorf("got %q", got)
	}
}

func TestResolveVariables_BraceOnly(t *testing.T) {
	m := newTestMacro(map[string]any{"y": 7})
	got, err := ResolveVariables("{y}+1", m)
	if err != nil {
		t.Fatal(err)
	}
	if got != "7+1" {
		t.Errorf("got %q", got)
	}
}

func TestResolveVariables_NilMacro(t *testing.T) {
	got, err := ResolveVariables("${x}", nil)
	if err != nil {
		t.Fatal(err)
	}
	if got != "${x}" {
		t.Errorf("nil macro should return original, got %q", got)
	}
}

func TestResolveVariables_UnknownVariable(t *testing.T) {
	m := newTestMacro(nil)
	got, err := ResolveVariables("${unknown}", m)
	if err != nil {
		t.Fatal(err)
	}
	if got != "${unknown}" {
		t.Errorf("unknown var should stay, got %q", got)
	}
}

func TestParseVariableReference(t *testing.T) {
	refs := ParseVariableReference("${a} and {b} and ${c}")
	names := make(map[string]bool)
	for _, r := range refs {
		names[r] = true
	}
	for _, want := range []string{"a", "b", "c"} {
		if !names[want] {
			t.Errorf("missing %q in %v", want, refs)
		}
	}
}

func TestEvaluateExpression_BasicArithmetic(t *testing.T) {
	m := newTestMacro(nil)
	tests := []struct {
		expr string
		want float64
	}{
		{"1+2", 3},
		{"10-3", 7},
		{"4*5", 20},
		{"10/2", 5},
		{"2**3", 8},
		{"(1+2)*3", 9},
	}
	for _, tt := range tests {
		t.Run(tt.expr, func(t *testing.T) {
			result, err := EvaluateExpression(tt.expr, m)
			if err != nil {
				t.Fatal(err)
			}
			f, ok := result.(float64)
			if !ok {
				t.Fatalf("expected float64, got %T", result)
			}
			if math.Abs(f-tt.want) > 0.001 {
				t.Errorf("got %f, want %f", f, tt.want)
			}
		})
	}
}

func TestEvaluateExpression_WithVariables(t *testing.T) {
	m := newTestMacro(map[string]any{"x": 10, "y": 5})
	result, err := EvaluateExpression("${x}+${y}", m)
	if err != nil {
		t.Fatal(err)
	}
	f := result.(float64)
	if math.Abs(f-15) > 0.001 {
		t.Errorf("got %f, want 15", f)
	}
}

func TestEvaluateExpression_Functions(t *testing.T) {
	m := newTestMacro(nil)
	result, err := EvaluateExpression("sqrt(16)", m)
	if err != nil {
		t.Fatal(err)
	}
	f := result.(float64)
	if math.Abs(f-4) > 0.001 {
		t.Errorf("sqrt(16) = %f, want 4", f)
	}
}

func TestEvaluateExpression_DivisionByZero(t *testing.T) {
	m := newTestMacro(nil)
	_, err := EvaluateExpression("1/0", m)
	if err == nil {
		t.Error("expected division by zero error")
	}
}

func TestResolveInt(t *testing.T) {
	m := newTestMacro(map[string]any{"count": 42})
	tests := []struct {
		name  string
		value any
		want  int
	}{
		{"int", 5, 5},
		{"float64", 3.7, 3},
		{"string_literal", "10", 10},
		{"string_variable", "${count}", 42},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ResolveInt(tt.value, m)
			if err != nil {
				t.Fatal(err)
			}
			if got != tt.want {
				t.Errorf("got %d, want %d", got, tt.want)
			}
		})
	}
}

func TestResolveInt_UnresolvedVariable(t *testing.T) {
	m := newTestMacro(nil)
	_, err := ResolveInt("${missing}", m)
	if err == nil {
		t.Error("expected error for unresolved variable")
	}
}

func TestResolveInt_UnsupportedType(t *testing.T) {
	m := newTestMacro(nil)
	_, err := ResolveInt([]int{1}, m)
	if err == nil {
		t.Error("expected error for unsupported type")
	}
}

func TestResolveFloat(t *testing.T) {
	m := newTestMacro(map[string]any{"pi": 3.14})
	tests := []struct {
		name  string
		value any
		want  float64
	}{
		{"float64", 2.5, 2.5},
		{"int", 3, 3.0},
		{"string_literal", "1.5", 1.5},
		{"string_variable", "${pi}", 3.14},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ResolveFloat(tt.value, m)
			if err != nil {
				t.Fatal(err)
			}
			if math.Abs(got-tt.want) > 0.001 {
				t.Errorf("got %f, want %f", got, tt.want)
			}
		})
	}
}

func TestResolveString(t *testing.T) {
	m := newTestMacro(map[string]any{"name": "world"})
	got, err := ResolveString("hello ${name}", m)
	if err != nil {
		t.Fatal(err)
	}
	if got != "hello world" {
		t.Errorf("got %q", got)
	}
	got2, err := ResolveString(42, m)
	if err != nil {
		t.Fatal(err)
	}
	if got2 != "42" {
		t.Errorf("got %q", got2)
	}
}

func TestResolveSearchAreaCoords(t *testing.T) {
	m := newTestMacro(map[string]any{"lx": 10})
	lx, ty, rx, by, err := ResolveSearchAreaCoords("${lx}", 20, 100, 200, m)
	if err != nil {
		t.Fatal(err)
	}
	if lx != 10 || ty != 20 || rx != 100 || by != 200 {
		t.Errorf("got %d,%d,%d,%d", lx, ty, rx, by)
	}
}
