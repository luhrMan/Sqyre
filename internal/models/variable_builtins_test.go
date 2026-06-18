package models

import (
	"reflect"
	"testing"
)

func TestMonitorBuiltinVarNames(t *testing.T) {
	got := MonitorBuiltinVarNames(2)
	want := []string{"monitor1Width", "monitor1Height", "monitor2Width", "monitor2Height"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("MonitorBuiltinVarNames(2) = %v, want %v", got, want)
	}

	gotZero := MonitorBuiltinVarNames(0)
	wantOne := []string{"monitor1Width", "monitor1Height"}
	if !reflect.DeepEqual(gotZero, wantOne) {
		t.Fatalf("MonitorBuiltinVarNames(0) = %v, want %v", gotZero, wantOne)
	}
}
