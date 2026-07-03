package actiondialog

import (
	"reflect"
	"testing"
)

func TestFilterRunMacroCompletionOptions(t *testing.T) {
	macros := []string{"Alpha", "beta-run", "GammaTask"}

	tests := []struct {
		name  string
		query string
		want  []string
	}{
		{
			name:  "empty query returns all options",
			query: "",
			want:  []string{"Alpha", "beta-run", "GammaTask"},
		},
		{
			name:  "filters case insensitive contains",
			query: "ta",
			want:  []string{"beta-run", "GammaTask"},
		},
		{
			name:  "no matches returns empty slice",
			query: "zzz",
			want:  []string{},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := filterRunMacroCompletionOptions(tc.query, macros)
			if !reflect.DeepEqual(got, tc.want) {
				t.Fatalf("filterRunMacroCompletionOptions(%q) = %v, want %v", tc.query, got, tc.want)
			}
		})
	}
}
