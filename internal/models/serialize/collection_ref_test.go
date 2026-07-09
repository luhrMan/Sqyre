package serialize

import (
	"Sqyre/internal/models/actions"
	"testing"
)

func TestActionToMap_CollectionRef(t *testing.T) {
	ref := actions.NewCollectionRef("Demo", "grid", 1, 2, 3, 4)
	is := actions.NewImageSearch("s", nil, nil, ref, 1, 1, 0.95, 5)
	m, err := ActionToMap(is)
	if err != nil {
		t.Fatal(err)
	}
	sa, ok := m["searcharea"].(string)
	if !ok || sa != "Demo~grid@1,2-3,4" {
		t.Fatalf("searcharea = %#v", m["searcharea"])
	}
}

func TestCreateActionFromMap_CollectionRef(t *testing.T) {
	raw := map[string]any{
		"type":       "move",
		"point":      "Prog~bag@2,3-2,3",
		"smooth":     false,
	}
	action, err := ViperSerializer.CreateActionFromMap(raw, nil)
	if err != nil {
		t.Fatal(err)
	}
	mv, ok := action.(*actions.Move)
	if !ok {
		t.Fatalf("type = %T", action)
	}
	if !mv.Point.IsCollection() || mv.Point.Name() != "bag" {
		t.Fatalf("Point = %q", mv.Point)
	}
	r1, c1, r2, c2, ok := mv.Point.CellRange()
	if !ok || r1 != 2 || c1 != 3 || r2 != 2 || c2 != 3 {
		t.Fatalf("range = %d,%d-%d,%d", r1, c1, r2, c2)
	}
}
