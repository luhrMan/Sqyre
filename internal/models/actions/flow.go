package actions

import "errors"

// ErrBreak, ErrContinue, and ErrStopped are returned by the executor for flow control
// or user-initiated cancellation. Iterable containers (Loop, ForEachRow, ImageSearch
// match loop) consume ErrBreak and ErrContinue; ErrStopped propagates to halt execution.
var (
	ErrBreak    = errors.New("break")
	ErrContinue = errors.New("continue")
	ErrStopped  = errors.New("stopped")
)

// IsFlowControl reports whether err is ErrBreak or ErrContinue.
func IsFlowControl(err error) bool {
	return errors.Is(err, ErrBreak) || errors.Is(err, ErrContinue)
}

// IsStopped reports whether err indicates the user stopped macro execution.
func IsStopped(err error) bool {
	return errors.Is(err, ErrStopped)
}
