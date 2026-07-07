package validation

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"testing"
)

func TestValidateAction_pauseRequiresContinueKey(t *testing.T) {
	err := ValidateAction(actions.NewPause("", nil, false), nil)
	if err == nil {
		t.Fatal("expected error for missing continue key")
	}
}

func TestValidateAction_keyRequiresKey(t *testing.T) {
	err := ValidateAction(actions.NewKey("", true), nil)
	if err == nil {
		t.Fatal("expected error for missing key")
	}
}

func TestValidateAction_calculateRequiresExpression(t *testing.T) {
	err := ValidateAction(actions.NewCalculate("", "out"), nil)
	if err == nil {
		t.Fatal("expected error for empty expression")
	}
}

func TestValidateAction_setVariableRequiresName(t *testing.T) {
	err := ValidateAction(actions.NewSetVariable("", "1"), nil)
	if err == nil {
		t.Fatal("expected error for empty variable name")
	}
}

func TestValidateAction_calculateValidExpression(t *testing.T) {
	m := &models.Macro{Name: "test"}
	m.InitRuntimeVariables()
	err := ValidateAction(actions.NewCalculate("1 + 2", "sum"), m)
	if err != nil {
		t.Fatalf("expected valid expression: %v", err)
	}
}
