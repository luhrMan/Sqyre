package services

import (
	"testing"

	"Sqyre/internal/models"
)

func TestResolveIntWithOverrides_ImagePixelWidth(t *testing.T) {
	overrides := map[string]any{
		"ImagePixelWidth":  64,
		"ImagePixelHeight": 32,
	}
	n, err := resolveIntWithOverrides("${ImagePixelWidth}", nil, overrides)
	if err != nil {
		t.Fatal(err)
	}
	if n != 64 {
		t.Fatalf("got %d want 64", n)
	}

	n, err = resolveIntWithOverrides("${ImagePixelWidth}/2", nil, overrides)
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
