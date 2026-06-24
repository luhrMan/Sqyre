package actions

import (
	"strings"
	"testing"
)

func TestFormatFloatUpTo2Decimals(t *testing.T) {
	tests := []struct {
		in   float64
		want string
	}{
		{0.9, "0.9"},
		{0.95, "0.95"},
		{1, "1"},
		{0.949999988, "0.95"},
		{0.01, "0.01"},
	}
	for _, tt := range tests {
		if got := formatFloatUpTo2Decimals(tt.in); got != tt.want {
			t.Errorf("formatFloatUpTo2Decimals(%v) = %q, want %q", tt.in, got, tt.want)
		}
	}
}

func TestImageSearch_String_toleranceFormatting(t *testing.T) {
	is := NewImageSearch("S", nil, []string{"a"}, NewCoordinateRef("prog", "R"), 1, 1, 0.95, 0)
	if got := is.String(); !strings.Contains(got, "Tolerance: 0.95") {
		t.Errorf("String() = %q, want tolerance formatted to 2 decimals", got)
	}
	is.Tolerance = 0.9
	if got := is.String(); !strings.Contains(got, "Tolerance: 0.9") {
		t.Errorf("String() = %q, want tolerance without trailing zero", got)
	}
}
