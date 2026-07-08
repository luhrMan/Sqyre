package ui

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

func TestResolveCoordinateRefKey(t *testing.T) {
	program := &models.Program{Name: "prog-a"}
	getKeys := func(*models.Program) []string {
		return []string{"alpha", "beta", "gamma"}
	}

	ref := actions.NewCoordinateRef("prog-a", "beta")
	key, ok := resolveCoordinateRefKey(ref, program, getKeys)
	if !ok || key != "beta" {
		t.Fatalf("resolveCoordinateRefKey() = %q, %v; want beta, true", key, ok)
	}

	other := &models.Program{Name: "prog-b"}
	if key, ok := resolveCoordinateRefKey(ref, other, getKeys); ok {
		t.Fatalf("expected no match for other program, got %q", key)
	}

	legacy := actions.CoordinateRef("gamma")
	key, ok = resolveCoordinateRefKey(legacy, program, getKeys)
	if !ok || key != "gamma" {
		t.Fatalf("legacy resolve = %q, %v; want gamma, true", key, ok)
	}

	missing := actions.NewCoordinateRef("prog-a", "missing")
	if _, ok := resolveCoordinateRefKey(missing, program, getKeys); ok {
		t.Fatal("expected no match for missing key")
	}
}
