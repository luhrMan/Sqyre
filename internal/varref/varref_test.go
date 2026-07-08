package varref

import (
	"reflect"
	"sort"
	"testing"
)

func TestSegments(t *testing.T) {
	tests := []struct {
		in   string
		want []Segment
	}{
		{in: "hello", want: []Segment{{Text: "hello"}}},
		{
			in:   "${count}",
			want: []Segment{{Text: "${count}", IsRef: true, Name: "count"}},
		},
		{
			in: "x={a}+${b}",
			want: []Segment{
				{Text: "x="},
				{Text: "{a}", IsRef: true, Name: "a"},
				{Text: "+"},
				{Text: "${b}", IsRef: true, Name: "b"},
			},
		},
		{in: "no ref ${incomplete", want: []Segment{{Text: "no ref ${incomplete"}}},
	}
	for _, tt := range tests {
		got := Segments(tt.in)
		if len(got) != len(tt.want) {
			t.Fatalf("Segments(%q) len=%d, want %d: %+v", tt.in, len(got), len(tt.want), got)
		}
		for i := range got {
			if got[i] != tt.want[i] {
				t.Fatalf("Segments(%q)[%d] = %+v, want %+v", tt.in, i, got[i], tt.want[i])
			}
		}
	}
}

func TestContains(t *testing.T) {
	if !Contains("${x}") {
		t.Fatal("expected dollar ref")
	}
	if !Contains("{x}") {
		t.Fatal("expected brace ref")
	}
	if Contains("plain") {
		t.Fatal("expected no ref")
	}
	if Contains("") {
		t.Fatal("empty string has no ref")
	}
}

func TestNames(t *testing.T) {
	got := Names("x=${a}+{b}")
	sort.Strings(got)
	if want := []string{"a", "b"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("Names = %v, want %v", got, want)
	}
	// ${x} contributes only "x" even though {x} is a substring (dedup).
	if got := Names("${x}"); !reflect.DeepEqual(got, []string{"x"}) {
		t.Fatalf("Names(${x}) = %v, want [x]", got)
	}
	if got := Names("plain"); len(got) != 0 {
		t.Fatalf("Names(plain) = %v, want empty", got)
	}
}

func TestReferences(t *testing.T) {
	if !References("value=${Count}", "count") {
		t.Fatal("expected case-insensitive dollar match")
	}
	if !References("value={ count }", "count") {
		t.Fatal("expected brace match tolerating spaces")
	}
	if References("price=$100", "100") {
		t.Fatal("$100 is not a {100} reference")
	}
	if References("", "x") || References("text", "") {
		t.Fatal("empty inputs never match")
	}
}

func TestRename(t *testing.T) {
	if got := Rename("a=${Old}+{old}", "old", "new"); got != "a=${new}+{new}" {
		t.Fatalf("Rename = %q, want %q", got, "a=${new}+{new}")
	}
	// A bare {old} not preceded by '$' is renamed; the '$' style is preserved.
	if got := Rename("${old}", "old", "new"); got != "${new}" {
		t.Fatalf("Rename dollar = %q, want ${new}", got)
	}
	if got := Rename("", "old", "new"); got != "" {
		t.Fatalf("Rename empty = %q", got)
	}
}
