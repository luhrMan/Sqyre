package actions

import "testing"

func TestCoordinateRef(t *testing.T) {
	ref := NewCoordinateRef("My Program", "Main area")
	if ref.Program() != "My Program" || ref.Name() != "Main area" {
		t.Fatalf("ref parts: program=%q name=%q", ref.Program(), ref.Name())
	}
	if ref.DisplayLabel() != "My Program~Main area" {
		t.Fatalf("DisplayLabel() = %q", ref.DisplayLabel())
	}

	legacy := CoordinateRef("legacy-only")
	if legacy.Program() != "" || legacy.Name() != "legacy-only" {
		t.Fatalf("legacy ref: program=%q name=%q", legacy.Program(), legacy.Name())
	}
	if legacy.DisplayLabel() != "legacy-only" {
		t.Fatalf("legacy DisplayLabel() = %q", legacy.DisplayLabel())
	}
}
