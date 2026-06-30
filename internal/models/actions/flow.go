package actions

import "errors"

// ErrBreak and ErrContinue are returned by the executor when a break or continue
// action runs. Iterable containers (Loop, ForEachRow, ImageSearch match loop)
// consume them; other containers propagate upward.
var (
	ErrBreak    = errors.New("break")
	ErrContinue = errors.New("continue")
)

// IsFlowControl reports whether err is ErrBreak or ErrContinue.
func IsFlowControl(err error) bool {
	return errors.Is(err, ErrBreak) || errors.Is(err, ErrContinue)
}
