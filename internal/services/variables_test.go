package services

import (
	"testing"

	"Sqyre/internal/models"
)

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
}
