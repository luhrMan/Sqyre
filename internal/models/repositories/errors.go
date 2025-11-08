package repositories

import "errors"

// Repository error definitions
var (
	// ErrNotFound is returned when a model with the specified key does not exist
	ErrNotFound = errors.New("model not found")

	// ErrInvalidKey is returned when an empty or invalid key is provided
	ErrInvalidKey = errors.New("invalid key: cannot be empty")

	// ErrSaveFailed is returned when persisting data to disk fails
	ErrSaveFailed = errors.New("failed to save to disk")

	// ErrLoadFailed is returned when loading data from disk fails
	ErrLoadFailed = errors.New("failed to load from disk")

	// ErrDecodeFailed is returned when decoding a model fails
	ErrDecodeFailed = errors.New("failed to decode model")
)
