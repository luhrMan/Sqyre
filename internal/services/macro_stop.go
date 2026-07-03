package services

import (
	"Sqyre/internal/models/actions"
)

func checkMacroStop() error {
	if macroStopRequested.Load() {
		return actions.ErrStopped
	}
	return nil
}

func interruptibleSleep(ms int) error {
	if ms <= 0 {
		return checkMacroStop()
	}
	const chunkMs = 50
	remaining := ms
	for remaining > 0 {
		if err := checkMacroStop(); err != nil {
			return err
		}
		step := min(chunkMs, remaining)
		getAutomationBackend().MilliSleep(step)
		remaining -= step
	}
	return nil
}
