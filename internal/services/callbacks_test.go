package services

import (
	"testing"
)

func TestRunOnMainThread_DefaultIsSynchronous(t *testing.T) {
	called := false
	RunOnMainThread(func() { called = true })
	if !called {
		t.Error("default RunOnMainThread should execute synchronously")
	}
}

func TestRunOnMainThreadAndWait_DefaultIsSynchronous(t *testing.T) {
	called := false
	RunOnMainThreadAndWait(func() { called = true })
	if !called {
		t.Error("default RunOnMainThreadAndWait should execute synchronously")
	}
}

func TestActivityReporter_DefaultNoop(t *testing.T) {
	r := GetActivityReporter()
	// should not panic
	r.Show()
	r.Start()
	r.Stop()
	r.Hide()
}

func TestProgressReporter_DefaultNoop(t *testing.T) {
	r := GetProgressReporter()
	// should not panic
	r.SetValue(0.5)
	r.Show()
	r.Hide()
}

type mockActivity struct {
	started, stopped, shown, hidden bool
}

func (m *mockActivity) Start() { m.started = true }
func (m *mockActivity) Stop()  { m.stopped = true }
func (m *mockActivity) Show()  { m.shown = true }
func (m *mockActivity) Hide()  { m.hidden = true }

func TestSetActivityReporter(t *testing.T) {
	orig := GetActivityReporter()
	defer SetActivityReporter(orig)

	mock := &mockActivity{}
	SetActivityReporter(mock)

	GetActivityReporter().Start()
	GetActivityReporter().Stop()
	GetActivityReporter().Show()
	GetActivityReporter().Hide()

	if !mock.started || !mock.stopped || !mock.shown || !mock.hidden {
		t.Errorf("mock activity: started=%v stopped=%v shown=%v hidden=%v",
			mock.started, mock.stopped, mock.shown, mock.hidden)
	}
}

func TestBoolPreference_DefaultReturnsFallback(t *testing.T) {
	if BoolPreference("anything", true) != true {
		t.Error("default BoolPreference should return fallback=true")
	}
	if BoolPreference("anything", false) != false {
		t.Error("default BoolPreference should return fallback=false")
	}
}
