package validation

import "errors"

var (
	ErrEmptyName       = errors.New("name cannot be empty")
	ErrInvalidVariable = errors.New("invalid variable name")
	ErrNotPositiveInt  = errors.New("must be a positive integer")
	ErrNegativeInt     = errors.New("must be zero or greater")
)
