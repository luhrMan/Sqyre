package custom_widgets

import (
	"sync"
	"time"

	"fyne.io/fyne/v2"
)

// DefaultSearchDebounce is the quiet period used for search/filter inputs so a
// burst of keystrokes triggers only one expensive rebuild.
const DefaultSearchDebounce = 180 * time.Millisecond

// Debouncer coalesces rapid calls (e.g. search box keystrokes) so the wrapped
// work runs once after input settles. The scheduled action runs on the Fyne UI
// thread via fyne.Do, so callers may safely mutate widgets inside it.
type Debouncer struct {
	delay time.Duration
	mu    sync.Mutex
	timer *time.Timer
}

// NewDebouncer returns a Debouncer that waits delay after the last Call before
// running the scheduled function.
func NewDebouncer(delay time.Duration) *Debouncer {
	return &Debouncer{delay: delay}
}

// Call schedules fn to run after the debounce delay, cancelling any run that was
// previously scheduled but has not yet fired. fn executes on the Fyne UI thread.
func (d *Debouncer) Call(fn func()) {
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.timer != nil {
		d.timer.Stop()
	}
	d.timer = time.AfterFunc(d.delay, func() {
		fyne.Do(fn)
	})
}

// Stop cancels any pending scheduled run.
func (d *Debouncer) Stop() {
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.timer != nil {
		d.timer.Stop()
		d.timer = nil
	}
}
