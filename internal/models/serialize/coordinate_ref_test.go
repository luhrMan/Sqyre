package serialize

import (
	"Sqyre/internal/models/actions"
	"testing"
)

func TestActionToMap_SearchAreaRefOnly(t *testing.T) {
	ref := actions.NewCoordinateRef("Demo", "Main area")
	is := actions.NewImageSearch("s", nil, nil, ref, 0.95, 5)
	m, err := ActionToMap(is)
	if err != nil {
		t.Fatal(err)
	}
	sa, ok := m["searcharea"].(string)
	if !ok || sa != "Demo~Main area" {
		t.Fatalf("searcharea = %#v, want string ref", m["searcharea"])
	}
}

func TestCreateActionFromMap_LegacySearchAreaMapUsesNameRef(t *testing.T) {
	raw := map[string]any{
		"type": "findpixel",
		"name": "fp",
		"searcharea": map[string]any{
			"name":    "legacy-area",
			"leftx":   10,
			"topy":    20,
			"rightx":  100,
			"bottomy": 200,
		},
		"targetcolor": "ffffff",
	}
	action, err := ViperSerializer.CreateActionFromMap(raw, nil)
	if err != nil {
		t.Fatal(err)
	}
	fp, ok := action.(*actions.FindPixel)
	if !ok {
		t.Fatalf("type = %T", action)
	}
	if fp.SearchArea.Name() != "legacy-area" {
		t.Fatalf("SearchArea ref name = %q", fp.SearchArea.Name())
	}
}

func TestCreateActionFromMap_SearchAreaStringRef(t *testing.T) {
	raw := map[string]any{
		"type":       "ocr",
		"name":       "o",
		"target":     "text",
		"searcharea": "Program~Area",
	}
	action, err := ViperSerializer.CreateActionFromMap(raw, nil)
	if err != nil {
		t.Fatal(err)
	}
	oc, ok := action.(*actions.Ocr)
	if !ok {
		t.Fatalf("type = %T", action)
	}
	if oc.SearchArea.String() != "Program~Area" {
		t.Fatalf("SearchArea = %q", oc.SearchArea)
	}
}
