package ui

import (
	"testing"

	"fyne.io/fyne/v2"
)

func TestClampWindowSizeToScreen(t *testing.T) {
	screen := fyne.NewSize(1024, 768)

	tests := []struct {
		name string
		in   fyne.Size
		want fyne.Size
	}{
		{
			name: "within screen unchanged",
			in:   fyne.NewSize(800, 600),
			want: fyne.NewSize(800, 600),
		},
		{
			name: "oversized clamped to margin",
			in:   fyne.NewSize(2000, 1500),
			want: fyne.NewSize(992, 736),
		},
		{
			name: "undersized raised to minimum",
			in:   fyne.NewSize(100, 100),
			want: fyne.NewSize(400, 300),
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := clampWindowSizeToScreen(tc.in, screen)
			if got != tc.want {
				t.Fatalf("clampWindowSizeToScreen(%v) = %v, want %v", tc.in, got, tc.want)
			}
		})
	}
}
