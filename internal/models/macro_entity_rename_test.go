package models

import (
	"testing"

	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
)

func TestRenameProgramEntity_point(t *testing.T) {
	m := NewMacro("t", 0, nil)
	move := actions.NewMove(actions.NewCoordinateRef("prog", "old"), false)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{move})

	if !m.RenameProgramEntity(ProgramEntityPoint, "prog", "old", "new") {
		t.Fatal("expected changes")
	}
	if move.Point.Name() != "new" || move.Point.Program() != "prog" {
		t.Fatalf("point ref = %q", move.Point)
	}
}

func TestRenameProgramEntity_searchArea(t *testing.T) {
	m := NewMacro("t", 0, nil)
	is := actions.NewImageSearch("s", nil, nil, actions.NewCoordinateRef("prog", "box"), 1, 1, 0.95, 0)
	ocr := actions.NewOcr("o", "t", actions.NewCoordinateRef("prog", "box"))
	fp := actions.NewFindPixel("f", actions.NewCoordinateRef("prog", "box"), "ffffff", 0)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{is, ocr, fp})

	if !m.RenameProgramEntity(ProgramEntitySearchArea, "prog", "box", "region") {
		t.Fatal("expected changes")
	}
	for _, ref := range []actions.CoordinateRef{is.SearchArea, ocr.SearchArea, fp.SearchArea} {
		if ref.Name() != "region" {
			t.Fatalf("search area ref = %q", ref)
		}
	}
}

func TestRenameProgramEntity_itemWithVariant(t *testing.T) {
	m := NewMacro("t", 0, nil)
	target := "prog" + config.ProgramDelimiter + "potion" + config.ProgramDelimiter + "glow"
	is := actions.NewImageSearch("s", nil, []string{target}, actions.CoordinateRef(""), 1, 1, 0.95, 0)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{is})

	if !m.RenameProgramEntity(ProgramEntityItem, "prog", "potion", "health") {
		t.Fatal("expected changes")
	}
	want := "prog" + config.ProgramDelimiter + "health" + config.ProgramDelimiter + "glow"
	if is.Targets[0] != want {
		t.Fatalf("target = %q, want %q", is.Targets[0], want)
	}
}

func TestRenameProgramEntity_otherProgramUntouched(t *testing.T) {
	m := NewMacro("t", 0, nil)
	move := actions.NewMove(actions.NewCoordinateRef("other", "old"), false)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{move})

	if m.RenameProgramEntity(ProgramEntityPoint, "prog", "old", "new") {
		t.Fatal("expected no changes")
	}
	if move.Point.Name() != "old" {
		t.Fatalf("point ref changed unexpectedly: %q", move.Point)
	}
}

func TestRenameProgram(t *testing.T) {
	m := NewMacro("t", 0, nil)
	target := "old" + config.ProgramDelimiter + "item"
	is := actions.NewImageSearch("s", nil, []string{target}, actions.NewCoordinateRef("old", "area"), 1, 1, 0.95, 0)
	move := actions.NewMove(actions.NewCoordinateRef("old", "pt"), false)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{is, move})

	if !m.RenameProgram("old", "new") {
		t.Fatal("expected changes")
	}
	if move.Point.Program() != "new" {
		t.Fatalf("move program = %q", move.Point.Program())
	}
	if is.SearchArea.Program() != "new" {
		t.Fatalf("search area program = %q", is.SearchArea.Program())
	}
	wantTarget := "new" + config.ProgramDelimiter + "item"
	if is.Targets[0] != wantTarget {
		t.Fatalf("target = %q, want %q", is.Targets[0], wantTarget)
	}
}

func TestRenameMacroReference(t *testing.T) {
	m := NewMacro("t", 0, nil)
	rm := actions.NewRunMacro("helper")
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{rm})

	if !m.RenameMacroReference("helper", "utility") {
		t.Fatal("expected changes")
	}
	if rm.MacroName != "utility" {
		t.Fatalf("MacroName = %q", rm.MacroName)
	}
}
