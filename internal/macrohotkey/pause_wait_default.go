//go:build !linux && !nohook

package macrohotkey

import (
	"slices"
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
	services.BeginMacroPauseWait(slices.Equal(keys, []string{"esc"}))
	defer services.EndMacroPauseWait()

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

	onMatch := func() {
		if opts.OnMatch != nil {
			opts.OnMatch()
		}
		signalDone()
	}

	if slices.Equal(keys, []string{"esc"}) {
		unregisterEsc := RegisterEscapeHandler(onMatch)
		defer unregisterEsc()
	} else {
		cb := func(hook.Event) {
			onMatch()
			go hook.Unregister(hook.KeyDown, keys)
		}
		hook.Register(hook.KeyDown, keys, cb)
		defer func() {
			go hook.Unregister(hook.KeyDown, keys)
		}()
	}

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
