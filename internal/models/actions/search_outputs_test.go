package actions_test

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestWaitTilFoundConfig_DisplayWaitMode(t *testing.T) {
	t.Run("instant", func(t *testing.T) {
		w := actions.WaitTilFoundConfig{}
		if got := w.DisplayWaitMode("instant"); got != "instant" {
			t.Fatalf("got %q, want instant", got)
		}
	})

	t.Run("wait until found", func(t *testing.T) {
		w := actions.WaitTilFoundConfig{RepeatMode: actions.RepeatWaitUntilFound, WaitTilFoundSeconds: 3}
		if got := w.DisplayWaitMode("instant"); got != "3 seconds or until found" {
			t.Fatalf("got %q, want %q", got, "3 seconds or until found")
		}
	})
}
