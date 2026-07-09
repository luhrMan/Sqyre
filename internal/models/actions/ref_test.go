package actions

import "testing"

func TestNewCollectionRef_andParse(t *testing.T) {
	ref := NewCollectionRef("prog", "grid", 2, 3, 1, 1)
	want := "prog~grid@1,1-2,3"
	if ref.String() != want {
		t.Fatalf("ref = %q want %q", ref, want)
	}
	if !ref.IsCollection() {
		t.Fatal("expected IsCollection")
	}
	if ref.Program() != "prog" || ref.Name() != "grid" {
		t.Fatalf("program/name = %q/%q", ref.Program(), ref.Name())
	}
	r1, c1, r2, c2, ok := ref.CellRange()
	if !ok || r1 != 1 || c1 != 1 || r2 != 2 || c2 != 3 {
		t.Fatalf("CellRange = %d,%d-%d,%d ok=%v", r1, c1, r2, c2, ok)
	}
}

func TestCoordinateRef_plainNotCollection(t *testing.T) {
	ref := NewCoordinateRef("prog", "pt")
	if ref.IsCollection() {
		t.Fatal("plain ref should not be collection")
	}
	if _, _, _, _, ok := ref.CellRange(); ok {
		t.Fatal("unexpected cell range")
	}
}

func TestWithEntityName_preservesRange(t *testing.T) {
	ref := NewCollectionRef("prog", "old", 1, 1, 2, 2)
	got := ref.WithEntityName("prog", "new")
	if got.String() != "prog~new@1,1-2,2" {
		t.Fatalf("got %q", got)
	}
}
