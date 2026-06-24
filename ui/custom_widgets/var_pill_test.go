package custom_widgets

import "testing"

func TestParseVarRefSegments(t *testing.T) {
	tests := []struct {
		in   string
		want []varTextSegment
	}{
		{
			in:   "hello",
			want: []varTextSegment{{text: "hello"}},
		},
		{
			in: "${count}",
			want: []varTextSegment{{
				text: "${count}", isRef: true, name: "count",
			}},
		},
		{
			in: "x=${a}+${b}",
			want: []varTextSegment{
				{text: "x="},
				{text: "${a}", isRef: true, name: "a"},
				{text: "+"},
				{text: "${b}", isRef: true, name: "b"},
			},
		},
		{
			in:   "no ref ${incomplete",
			want: []varTextSegment{{text: "no ref ${incomplete"}},
		},
	}
	for _, tt := range tests {
		got := parseVarRefSegments(tt.in)
		if len(got) != len(tt.want) {
			t.Fatalf("parseVarRefSegments(%q) len=%d, want %d: %+v", tt.in, len(got), len(tt.want), got)
		}
		for i := range got {
			if got[i].text != tt.want[i].text || got[i].isRef != tt.want[i].isRef || got[i].name != tt.want[i].name {
				t.Fatalf("parseVarRefSegments(%q)[%d] = %+v, want %+v", tt.in, i, got[i], tt.want[i])
			}
		}
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
}
