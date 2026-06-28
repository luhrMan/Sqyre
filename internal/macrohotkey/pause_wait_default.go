//go:build !linux && !nohook

package macrohotkey

import (
	"sync"
	"time"

	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
	"log"

	hook "github.com/luhrMan/gohook"
)

func waitForContinueKey(opts services.ContinueWaitOptions) error {
	keys := NormalizeContinueKey(opts.Keys)
	if err := ValidateContinueKey(keys); err != nil {
		return err
	}

	SuspendMacroHotkeys()
	defer ResumeMacroHotkeys()

	releaseGrab, err := grabContinueChord(keys, !opts.PassThrough)
	if err != nil {
		log.Printf("Pause: key grab unavailable (%v); continue key may pass through to other apps", err)
	}
	if releaseGrab != nil {
		defer releaseGrab()
	}

	done := make(chan struct{}, 1)
	var once sync.Once
	signalDone := func() {
		once.Do(func() {
			close(done)
		})
	}

	cb := func(hook.Event) {
		if opts.OnMatch != nil {
			opts.OnMatch()
		}
		signalDone()
		go hook.Unregister(hook.KeyDown, keys)
	}

	hook.Register(hook.KeyDown, keys, cb)
	defer func() {
		go hook.Unregister(hook.KeyDown, keys)
	}()

	stopPoll := time.NewTicker(50 * time.Millisecond)
	defer stopPoll.Stop()
	for {
		select {
		case <-done:
			return nil
		case <-stopPoll.C:
			if services.MacroStopPending() {
				return actions.ErrStopped
			}
		}
	}
}
