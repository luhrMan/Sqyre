package services

import (
	"errors"
	"testing"

	"Sqyre/internal/models/actions"
)

func TestRetryWhileNotFound_inactive(t *testing.T) {
	called := false
	err := retryWhileNotFound(actions.WaitTilFoundConfig{}, 100, func() (bool, error) {
		called = true
		return false, nil
	})
	if err != nil {
		t.Fatalf("retryWhileNotFound: %v", err)
	}
	if called {
		t.Fatal("expected no retries when wait-until-found is inactive")
	}
}

func TestRetryWhileNotFound_findsOnRetry(t *testing.T) {
	cfg := actions.WaitTilFoundConfig{
		WaitTilFound:           true,
		WaitTilFoundSeconds:    1,
		WaitTilFoundIntervalMs: 5,
	}
	attempts := 0
	err := retryWhileNotFound(cfg, 100, func() (bool, error) {
		attempts++
		return attempts >= 2, nil
	})
	if err != nil {
		t.Fatalf("retryWhileNotFound: %v", err)
	}
	if attempts < 2 {
		t.Fatalf("expected at least 2 attempts, got %d", attempts)
	}
}

func TestRetryWhileNotFound_propagatesRetryError(t *testing.T) {
	cfg := actions.WaitTilFoundConfig{
		WaitTilFound:           true,
		WaitTilFoundSeconds:    1,
		WaitTilFoundIntervalMs: 5,
	}
	want := errors.New("search failed")
	err := retryWhileNotFound(cfg, 100, func() (bool, error) {
		return false, want
	})
	if !errors.Is(err, want) {
		t.Fatalf("got err %v, want %v", err, want)
	}
}
