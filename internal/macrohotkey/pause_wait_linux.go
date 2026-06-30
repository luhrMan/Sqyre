//go:build linux && !nohook

package macrohotkey

import (
	"time"

	"Sqyre/internal/hookkeys"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"
	"log"
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

	reader, err := hookkeys.NewReader()
	if err != nil {
		return err
	}
	defer reader.Close()

	chordWasPressed := false
	stopPoll := time.NewTicker(50 * time.Millisecond)
	defer stopPoll.Stop()
	for range stopPoll.C {
		if services.MacroStopPending() {
			return actions.ErrStopped
		}
		pressed := hookkeys.ChordAllPressed(reader, keys)
		if pressed && !chordWasPressed {
			if opts.OnMatch != nil {
				opts.OnMatch()
			}
			return nil
		}
		chordWasPressed = pressed
	}
	return nil
}
