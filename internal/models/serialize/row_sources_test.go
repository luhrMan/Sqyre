package serialize

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestActionToMap_forEachRow_roundTrip(t *testing.T) {
	orig := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "a\nb", OutputVar: "x", IsFile: false},
		{Source: "c.txt", OutputVar: "y", IsFile: true, SkipBlankLines: true},
	}, []actions.ActionInterface{actions.NewWait(1)})

	m, err := ActionToMap(orig)
	if err != nil {
		t.Fatal(err)
	}
	if m["type"] != "foreachrow" {
		t.Fatalf("type = %v", m["type"])
	}
	back, err := ViperSerializer.CreateActionFromMap(m, nil)
	if err != nil {
		t.Fatal(err)
	}
	fer, ok := back.(*actions.ForEachRow)
	if !ok {
		t.Fatalf("got %T", back)
	}
	if fer.Name != "rows" || len(fer.Sources) != 2 {
		t.Fatalf("fer = %+v", fer)
	}
	if len(fer.GetSubActions()) != 1 {
		t.Fatalf("subactions = %d", len(fer.GetSubActions()))
	}
}
