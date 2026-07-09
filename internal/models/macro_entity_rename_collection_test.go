package models

import (
	"Sqyre/internal/models/actions"
	"testing"
)

func TestRenameProgramEntity_collection(t *testing.T) {
	m := NewMacro("t", 0, nil)
	move := actions.NewMove(actions.NewCollectionRef("prog", "old", 1, 1, 2, 2), false)
	is := actions.NewImageSearch("s", nil, nil, actions.NewCollectionRef("prog", "old", 1, 2, 1, 3), 1, 1, 0.95, 0)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{move, is})

	if !m.RenameProgramEntity(ProgramEntityCollection, "prog", "old", "new") {
		t.Fatal("expected changes")
	}
	if move.Point.String() != "prog~new@1,1-2,2" {
		t.Fatalf("move point = %q", move.Point)
	}
	if is.SearchArea.String() != "prog~new@1,2-1,3" {
		t.Fatalf("search area = %q", is.SearchArea)
	}
}

func TestRenameProgramEntity_collectionDoesNotRenamePoint(t *testing.T) {
	m := NewMacro("t", 0, nil)
	move := actions.NewMove(actions.NewCoordinateRef("prog", "old"), false)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{move})

	if m.RenameProgramEntity(ProgramEntityCollection, "prog", "old", "new") {
		t.Fatal("expected no changes for plain point")
	}
}

func TestRenameProgram_collectionRef(t *testing.T) {
	m := NewMacro("t", 0, nil)
	move := actions.NewMove(actions.NewCollectionRef("old", "grid", 1, 1, 1, 1), false)
	m.Root = actions.NewLoop(1, "root", []actions.ActionInterface{move})

	if !m.RenameProgram("old", "new") {
		t.Fatal("expected changes")
	}
	if move.Point.String() != "new~grid@1,1-1,1" {
		t.Fatalf("move point = %q", move.Point)
	}
}
