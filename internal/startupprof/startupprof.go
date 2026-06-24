// Package startupprof provides lightweight, opt-in startup timing.
//
// It is enabled by setting SQYRE_STARTUP_PROFILE=1. When disabled, Mark and
// Dump are near-zero cost so the instrumentation can stay in the hot startup
// path permanently.
//
// Marks are timestamped relative to two reference points:
//   - the true process start time (read from /proc on Linux), which captures
//     dynamic-linker and Go runtime/package-init cost incurred before main(); and
//   - the first Mark, for relative deltas between phases.
package startupprof

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"
)

var (
	mu      sync.Mutex
	enabled = os.Getenv("SQYRE_STARTUP_PROFILE") == "1"
	procT0  = estimateProcStart()
	marks   []mark
)

type mark struct {
	label string
	at    time.Time
}

// Enabled reports whether startup profiling is turned on.
func Enabled() bool { return enabled }

// Mark records a timestamped checkpoint. No-op unless profiling is enabled.
func Mark(label string) {
	if !enabled {
		return
	}
	mu.Lock()
	marks = append(marks, mark{label: label, at: time.Now()})
	mu.Unlock()
}

// Dump writes the collected marks as a table to stderr. No-op unless enabled.
func Dump() {
	if !enabled {
		return
	}
	mu.Lock()
	defer mu.Unlock()
	if len(marks) == 0 {
		return
	}

	var b strings.Builder
	b.WriteString("\n=== Sqyre startup profile ===\n")
	hasProc := !procT0.IsZero()
	if hasProc {
		fmt.Fprintf(&b, "%-42s %12s %12s\n", "phase", "since-exec", "delta")
	} else {
		fmt.Fprintf(&b, "%-42s %12s %12s\n", "phase", "since-main", "delta")
		procT0 = marks[0].at
	}

	prev := procT0
	for _, m := range marks {
		fmt.Fprintf(&b, "%-42s %10.1fms %10.1fms\n",
			m.label,
			float64(m.at.Sub(procT0).Microseconds())/1000.0,
			float64(m.at.Sub(prev).Microseconds())/1000.0,
		)
		prev = m.at
	}
	b.WriteString("=============================\n")
	fmt.Fprint(os.Stderr, b.String())
}

// estimateProcStart returns the wall-clock instant the process began executing.
//
// It computes elapsed-since-exec from two boot-relative clocks read at package
// init: /proc/self/stat field 22 (starttime, ticks since boot) and /proc/uptime
// (seconds since boot, ~10ms precision). This avoids the whole-second rounding
// of /proc/stat btime. Returns the zero time on non-Linux or parse failure.
func estimateProcStart() time.Time {
	now := time.Now()

	statData, err := os.ReadFile("/proc/self/stat")
	if err != nil {
		return time.Time{}
	}
	s := string(statData)
	// comm (field 2) is wrapped in parens and may contain spaces/parens;
	// fields after the last ')' start at field 3 (state).
	rparen := strings.LastIndexByte(s, ')')
	if rparen < 0 || rparen+2 >= len(s) {
		return time.Time{}
	}
	fields := strings.Fields(s[rparen+2:])
	// field 22 (starttime) -> index 19 in the post-')' slice (field 3 == index 0).
	const startTimeIdx = 19
	if len(fields) <= startTimeIdx {
		return time.Time{}
	}
	startTicks, err := strconv.ParseInt(fields[startTimeIdx], 10, 64)
	if err != nil {
		return time.Time{}
	}

	uptime, ok := readUptime()
	if !ok {
		return time.Time{}
	}

	const clkTck = 100.0 // Linux default SC_CLK_TCK
	startSinceBoot := float64(startTicks) / clkTck
	elapsed := uptime - startSinceBoot
	if elapsed < 0 {
		elapsed = 0
	}
	return now.Add(-time.Duration(elapsed * float64(time.Second)))
}

func readUptime() (float64, bool) {
	data, err := os.ReadFile("/proc/uptime")
	if err != nil {
		return 0, false
	}
	f := strings.Fields(string(data))
	if len(f) == 0 {
		return 0, false
	}
	v, err := strconv.ParseFloat(f[0], 64)
	if err != nil {
		return 0, false
	}
	return v, true
}
