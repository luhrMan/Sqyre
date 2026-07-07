package custom_widgets

import (
	"Sqyre/internal/services"
	"testing"
)

func TestParseVarRefSegments_ui(t *testing.T) {
	got := services.ParseVarRefSegments("x={a}+${b}")
	if len(got) != 4 {
		t.Fatalf("len=%d want 4", len(got))
	}
}

func TestBuildVarRefPillContent_usesNestedPills(t *testing.T) {
	known := map[string]bool{"count": true}
	content := BuildVarRefPillContent("x=${count}", known)
	if content.MinSize().Height > PillLineHeight()+2 {
		t.Fatalf("nested content height %v exceeds pill line %v", content.MinSize().Height, PillLineHeight())
	}
}

func TestVarEntry_shouldShowPills(t *testing.T) {
	e := NewVarEntry(nil)
	e.SetText("${x}")
	e.hasFocus = false
	if !e.shouldShowPills() {
		t.Fatal("expected pills when unfocused with var ref")
	}
	e.hasFocus = true
	if e.shouldShowPills() {
		t.Fatal("expected raw text while focused")
	}
	e.SetText("plain")
	if e.shouldShowPills() {
		t.Fatal("expected no pills without var refs")
	}
	e.SetText("{x}")
	e.hasFocus = false
	if !e.shouldShowPills() {
		t.Fatal("expected pills for brace ref")
	}
}
