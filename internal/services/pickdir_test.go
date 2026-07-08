package services

import "testing"

func TestRunChooser_Selection(t *testing.T) {
	path, ok, err := runChooser("printf", "%s", "/tmp/chosen")
	if err != nil {
		t.Fatalf("runChooser: %v", err)
	}
	if !ok || path != "/tmp/chosen" {
		t.Errorf("got path=%q ok=%v, want /tmp/chosen true", path, ok)
	}
}

func TestRunChooser_CancelledOnCleanNonZeroExit(t *testing.T) {
	_, ok, err := runChooser("sh", "-c", "exit 1")
	if err != nil {
		t.Fatalf("runChooser returned error for cancel: %v", err)
	}
	if ok {
		t.Error("expected ok=false on non-zero exit (user cancelled)")
	}
}

func TestRunChooser_EmptySelectionIsCancel(t *testing.T) {
	_, ok, err := runChooser("printf", "%s", "   ")
	if err != nil {
		t.Fatalf("runChooser: %v", err)
	}
	if ok {
		t.Error("expected ok=false when chooser prints only whitespace")
	}
}

func TestRunChooser_MissingBinaryIsError(t *testing.T) {
	_, _, err := runChooser("sqyre-no-such-binary-xyz")
	if err == nil {
		t.Fatal("expected error when chooser binary is missing")
	}
}
