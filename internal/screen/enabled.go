package screen

import (
	"slices"
	"sort"
	"strconv"
	"strings"
)

// EnabledMonitorsString returns the raw enabled-monitors preference (comma-separated indices).
// The UI wires this at startup; empty means all monitors.
var EnabledMonitorsString func() string

// EnabledMonitorIndices returns the monitor indices the user selected in settings.
// nil means all monitors are enabled (default when the preference is empty or invalid).
func EnabledMonitorIndices() []int {
	s := ""
	if EnabledMonitorsString != nil {
		s = strings.TrimSpace(EnabledMonitorsString())
	}
	if s == "" {
		return nil
	}
	n := NumDisplays()
	seen := make(map[int]bool)
	var out []int
	for part := range strings.SplitSeq(s, ",") {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}
		i, err := strconv.Atoi(part)
		if err != nil || i < 0 || i >= n {
			continue
		}
		if !seen[i] {
			seen[i] = true
			out = append(out, i)
		}
	}
	sort.Ints(out)
	if len(out) == 0 {
		return nil
	}
	return out
}

// IsMonitorEnabled reports whether the given display index is allowed by user settings.
func IsMonitorEnabled(displayIndex int) bool {
	enabled := EnabledMonitorIndices()
	if enabled == nil {
		return true
	}
	return slices.Contains(enabled, displayIndex)
}
