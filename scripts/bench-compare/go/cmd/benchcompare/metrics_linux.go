//go:build linux

package main

import (
	"os"
	"syscall"
	"time"
)

type sample struct {
	wall    time.Time
	cpuUser time.Duration
	cpuSys  time.Duration
	ioRead  uint64
	ioWrite uint64
	rssKb   uint64
	hwmKb   uint64
}

func sampleNow() sample {
	var ru syscall.Rusage
	_ = syscall.Getrusage(syscall.RUSAGE_SELF, &ru)
	r, w := ioBytes()
	rss, hwm := rssAndHwm()
	return sample{
		wall:    time.Now(),
		cpuUser: timevalDuration(ru.Utime),
		cpuSys:  timevalDuration(ru.Stime),
		ioRead:  r,
		ioWrite: w,
		rssKb:   rss,
		hwmKb:   hwm,
	}
}

func timevalDuration(tv syscall.Timeval) time.Duration {
	return time.Duration(tv.Sec)*time.Second + time.Duration(tv.Usec)*time.Microsecond
}

func ioBytes() (uint64, uint64) {
	b, err := os.ReadFile("/proc/self/io")
	if err != nil {
		return 0, 0
	}
	var read, write uint64
	for _, line := range splitLines(string(b)) {
		if v, ok := stripPrefix(line, "read_bytes: "); ok {
			read = parseU64(v)
		} else if v, ok := stripPrefix(line, "write_bytes: "); ok {
			write = parseU64(v)
		}
	}
	return read, write
}

func rssAndHwm() (uint64, uint64) {
	b, err := os.ReadFile("/proc/self/status")
	if err != nil {
		return 0, 0
	}
	var rss, hwm uint64
	for _, line := range splitLines(string(b)) {
		if v, ok := stripPrefix(line, "VmRSS:"); ok {
			rss = parseKb(v)
		} else if v, ok := stripPrefix(line, "VmHWM:"); ok {
			hwm = parseKb(v)
		}
	}
	return rss, hwm
}

func splitLines(s string) []string {
	out := make([]string, 0, 32)
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			out = append(out, s[start:i])
			start = i + 1
		}
	}
	if start < len(s) {
		out = append(out, s[start:])
	}
	return out
}

func stripPrefix(s, prefix string) (string, bool) {
	if len(s) >= len(prefix) && s[:len(prefix)] == prefix {
		return s[len(prefix):], true
	}
	return "", false
}

func parseU64(s string) uint64 {
	var n uint64
	for _, c := range s {
		if c >= '0' && c <= '9' {
			n = n*10 + uint64(c-'0')
		} else if n > 0 {
			break
		}
	}
	return n
}

func parseKb(s string) uint64 {
	return parseU64(s)
}
