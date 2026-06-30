package services

import "testing"

func TestParseVarRefSegments(t *testing.T) {
	tests := []struct {
		in   string
		want []VarRefSegment
	}{
		{in: "hello", want: []VarRefSegment{{Text: "hello"}}},
		{
			in: "${count}",
			want: []VarRefSegment{{
				Text: "${count}", IsRef: true, Name: "count",
			}},
		},
		{
			in: "x={a}+${b}",
			want: []VarRefSegment{
				{Text: "x="},
				{Text: "{a}", IsRef: true, Name: "a"},
				{Text: "+"},
				{Text: "${b}", IsRef: true, Name: "b"},
			},
		},
		{in: "no ref ${incomplete", want: []VarRefSegment{{Text: "no ref ${incomplete"}}},
	}
	for _, tt := range tests {
		got := ParseVarRefSegments(tt.in)
		if len(got) != len(tt.want) {
			t.Fatalf("ParseVarRefSegments(%q) len=%d, want %d: %+v", tt.in, len(got), len(tt.want), got)
		}
		for i := range got {
			if got[i].Text != tt.want[i].Text || got[i].IsRef != tt.want[i].IsRef || got[i].Name != tt.want[i].Name {
				t.Fatalf("ParseVarRefSegments(%q)[%d] = %+v, want %+v", tt.in, i, got[i], tt.want[i])
			}
		}
	}
}

func TestTextContainsVarRef(t *testing.T) {
	if !TextContainsVarRef("${x}") {
		t.Fatal("expected dollar ref")
	}
	if !TextContainsVarRef("{x}") {
		t.Fatal("expected brace ref")
	}
	if TextContainsVarRef("plain") {
		t.Fatal("expected no ref")
	}
}
