package hotkeytrigger

import (
	"sync"
	"time"
)

// PressTryAcquireLatch takes the press latch; false if already latched (blocks key-repeat).
func PressTryAcquireLatch(mu *sync.Mutex, latched *bool) bool {
	mu.Lock()
	defer mu.Unlock()
	if *latched {
		return false
	}
	*latched = true
	return true
}

// PressClearLatch releases the press latch after the chord is no longer held.
func PressClearLatch(mu *sync.Mutex, latched *bool) {
	mu.Lock()
	*latched = false
	mu.Unlock()
}

// WaitWhileAllPressed polls until allPressed returns false (e.g. chord released).
func WaitWhileAllPressed(allPressed func() bool, poll time.Duration) {
	for allPressed() {
		time.Sleep(poll)
	}
}

// RunAfterChordThenFullRelease invokes onFire after: (1) the chord is no longer all held down,
// then (2) every chord key is up (fullyReleased). Matches on-release hotkey semantics.
func RunAfterChordThenFullRelease(allPressed, fullyReleased func() bool, poll time.Duration, onFire func()) {
	for allPressed() {
		time.Sleep(poll)
	}
	for !fullyReleased() {
		time.Sleep(poll)
	}
	onFire()
}
