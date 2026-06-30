package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"time"
)

// retryWhileNotFound sleeps and calls retry until it returns true or the deadline passes.
// defaultIntervalMs is used when WaitTilFoundIntervalMs is unset. The interval grows
// adaptively up to 5× the initial value (capped at 2s) between attempts.
func retryWhileNotFound(cfg actions.WaitTilFoundConfig, defaultIntervalMs int, retry func() (found bool, err error)) error {
	if !cfg.Active() {
		return nil
	}
	deadline := time.Now().Add(time.Duration(cfg.WaitTilFoundSeconds) * time.Second)
	intervalMs := cfg.EffectiveIntervalMs(defaultIntervalMs)
	maxIntervalMs := intervalMs * 5
	if maxIntervalMs > 2000 {
		maxIntervalMs = 2000
	}
	if maxIntervalMs < intervalMs {
		maxIntervalMs = intervalMs
	}

	for time.Now().Before(deadline) {
		time.Sleep(time.Duration(intervalMs) * time.Millisecond)
		if err := checkMacroStop(); err != nil {
			return err
		}
		found, err := retry()
		if err != nil {
			return err
		}
		if found {
			return nil
		}
		if intervalMs < maxIntervalMs {
			next := intervalMs * 2
			if next > maxIntervalMs {
				next = maxIntervalMs
			}
			intervalMs = next
		}
	}
	return nil
}

func setCoordinateOutputs(macro *models.Macro, out actions.CoordinateOutputs, x, y int) {
	if macro == nil {
		return
	}
	if out.OutputXVariable != "" {
		setMacroVariable(macro, out.OutputXVariable, x)
	}
	if out.OutputYVariable != "" {
		setMacroVariable(macro, out.OutputYVariable, y)
	}
}
