package hotkeytrigger

import (
	"sync"
	"testing"
	"time"
)

func TestPressLatchBlocksRepeat(t *testing.T) {
	t.Parallel()
	var mu sync.Mutex
	var latched bool
	if !PressTryAcquireLatch(&mu, &latched) {
		t.Fatal("first latch should succeed")
	}
	if PressTryAcquireLatch(&mu, &latched) {
		t.Fatal("second latch while held should fail")
	}
	PressClearLatch(&mu, &latched)
	if latched {
		t.Fatal("latch should clear")
	}
	if !PressTryAcquireLatch(&mu, &latched) {
		t.Fatal("latch after clear should succeed")
	}
}

func TestRunAfterChordThenFullRelease(t *testing.T) {
	t.Parallel()
	var fired int
	apCalls := 0
	allPressed := func() bool {
		apCalls++
		return apCalls <= 2
	}
	frCalls := 0
	fullyReleased := func() bool {
		frCalls++
		return frCalls >= 2
	}
	RunAfterChordThenFullRelease(allPressed, fullyReleased, time.Microsecond, func() { fired++ })
	if fired != 1 || apCalls < 3 || frCalls < 2 {
		t.Fatalf("fired=%d apCalls=%d frCalls=%d", fired, apCalls, frCalls)
	}
}

func TestWaitWhileAllPressed(t *testing.T) {
	t.Parallel()
	n := 0
	allPressed := func() bool {
		if n < 2 {
			n++
			return true
		}
		return false
	}
	start := time.Now()
	WaitWhileAllPressed(allPressed, time.Millisecond)
	if time.Since(start) < time.Millisecond {
		t.Fatal("expected at least one sleep")
	}
	if n < 2 {
		t.Fatalf("expected poll loop to run, n=%d", n)
	}
}
