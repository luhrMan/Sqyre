//go:build !linux

package main

import "time"

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
	return sample{wall: time.Now()}
}
