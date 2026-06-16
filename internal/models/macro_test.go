package models

import "testing"

func TestParseHotkeyTrigger(t *testing.T) {
	t.Parallel()
	cases := []struct {
		in   string
		want HotkeyTrigger
	}{
		{"", HotkeyTriggerPress},
		{"press", HotkeyTriggerPress},
		{"PRESS", HotkeyTriggerPress},
		{" release ", HotkeyTriggerRelease},
		{"unknown", HotkeyTriggerPress},
	}
	for _, tc := range cases {
		if got := ParseHotkeyTrigger(tc.in); got != tc.want {
			t.Errorf("ParseHotkeyTrigger(%q) = %q want %q", tc.in, got, tc.want)
		}
	}
}

func TestHotkeyTriggerUILabelRoundTrip(t *testing.T) {
	t.Parallel()
	for _, tr := range []HotkeyTrigger{HotkeyTriggerPress, HotkeyTriggerRelease} {
		label := tr.UILabel()
		back := HotkeyTriggerFromUILabel(label)
		if back != tr {
			t.Errorf("%q UILabel %q round-trips to %q", tr, label, back)
		}
	}
}
