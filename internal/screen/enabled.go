package screen

import (
	"sort"
	"strconv"
	"strings"
)

// prefEnabledMonitorsKey must stay in sync with config.PrefEnabledMonitors.
const prefEnabledMonitorsKey = "enabled_monitors"

// StringPreference reads a string preference by key.
// Set to fyne.CurrentApp().Preferences().String by the UI layer;
// defaults to returning "" for headless/test use.
var StringPreference = func(key string) string { return "" }

// EnabledMonitorIndices returns the monitor indices the user selected in settings.
// nil means all monitors are enabled (default when the preference is empty or invalid).
func EnabledMonitorIndices() []int {
	s := strings.TrimSpace(StringPreference(prefEnabledMonitorsKey))
	if s == "" {
		return nil
	}
	n := NumDisplays()
	seen := make(map[int]bool)
	var out []int
	for _, part := range strings.Split(s, ",") {
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
	for _, i := range enabled {
		if i == displayIndex {
			return true
		}
	}
	return false
}
