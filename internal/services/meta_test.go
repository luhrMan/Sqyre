package services

import "testing"

func TestSanitizeMetaPurpose(t *testing.T) {
	tests := []struct {
		in   string
		want string
	}{
		{"searcharea", "searcharea"},
		{"cmask-Health Potion-v1", "cmask-Health-Potion-v1"},
		{"program~item~variant", "program-item-variant"},
		{"///", ""},
	}
	for _, tt := range tests {
		if got := sanitizeMetaPurpose(tt.in); got != tt.want {
			t.Errorf("sanitizeMetaPurpose(%q) = %q, want %q", tt.in, got, tt.want)
		}
	}
}
