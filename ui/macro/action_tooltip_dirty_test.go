package macro

import (
	"testing"

	"Sqyre/internal/models/actions"
)

func TestHasPendingChanges_detectsEditedField(t *testing.T) {
	t.Helper()
	click := actions.NewClick(actions.ClickButtonLeft, false)
	baseline, err := snapshotActionMap(click)
	if err != nil {
		t.Fatalf("snapshot: %v", err)
	}
	form := &tooltipEditForm{
		baseline: baseline,
		applyAction: func() error {
			click.Button = actions.ClickButtonRight
			return nil
		},
	}
	if !form.hasPendingChanges(click) {
		t.Fatal("expected pending changes after apply differs from baseline")
	}
	if click.Button != actions.ClickButtonLeft {
		t.Fatalf("expected action restored, got button %q", click.Button)
	}
}

func TestHasPendingChanges_noChanges(t *testing.T) {
	t.Helper()
	click := actions.NewClick(actions.ClickButtonLeft, false)
	baseline, err := snapshotActionMap(click)
	if err != nil {
		t.Fatalf("snapshot: %v", err)
	}
	form := &tooltipEditForm{
		baseline: baseline,
		applyAction: func() error {
			return nil
		},
	}
	if form.hasPendingChanges(click) {
		t.Fatal("expected no pending changes")
	}
}
