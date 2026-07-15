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

func TestValidateAction_setVariableRequiresName(t *testing.T) {
	err := ValidateAction(actions.NewSetVariable("", "1"), nil)
	if err == nil {
		t.Fatal("expected error for empty variable name")
	}
}

func TestValidateAction_setVariableExpression(t *testing.T) {
	m := &models.Macro{Name: "test"}
	m.InitRuntimeVariables()
	err := ValidateAction(actions.NewSetVariable("sum", "1 + 2"), m)
	if err != nil {
		t.Fatalf("expected valid expression: %v", err)
	}
	err = ValidateAction(actions.NewSetVariable("sum", "1 +"), m)
	if err == nil {
		t.Fatal("expected error for invalid expression")
	}
}
