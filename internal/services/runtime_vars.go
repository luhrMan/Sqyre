package services

import (
	"fmt"
	"sort"
	"sync"

	"Sqyre/internal/models"

	"fyne.io/fyne/v2"
)

var (
	runtimeVarsMu       sync.RWMutex
	runtimeVars         map[string]string
	runtimeVarsListener func()
)

// SetRuntimeVariablesListener is called on the UI thread when the live variable snapshot updates.
func SetRuntimeVariablesListener(fn func()) {
	runtimeVarsMu.Lock()
	defer runtimeVarsMu.Unlock()
	runtimeVarsListener = fn
}

// ClearRuntimeVariables resets the live variable snapshot shown during macro execution.
func ClearRuntimeVariables() {
	runtimeVarsMu.Lock()
	defer runtimeVarsMu.Unlock()
	runtimeVars = nil
}

// SnapshotRuntimeVariables copies the current macro variable store for the log UI.
func SnapshotRuntimeVariables(m *models.Macro) {
	runtimeVarsMu.Lock()
	defer runtimeVarsMu.Unlock()
	if m == nil || m.Variables == nil {
		runtimeVars = nil
		return
	}
	names := m.Variables.GetAll()
	if len(names) == 0 {
		runtimeVars = map[string]string{}
		return
	}
	sort.Strings(names)
	out := make(map[string]string, len(names))
	for _, name := range names {
		if v, ok := m.Variables.Get(name); ok {
			out[name] = fmt.Sprintf("%v", v)
		}
	}
	runtimeVars = out
	if runtimeVarsListener != nil {
		fyne.Do(runtimeVarsListener)
	}
}

// GetRuntimeVariables returns the latest snapshot (empty map if none).
func GetRuntimeVariables() map[string]string {
	runtimeVarsMu.RLock()
	defer runtimeVarsMu.RUnlock()
	if runtimeVars == nil {
		return map[string]string{}
	}
	out := make(map[string]string, len(runtimeVars))
	for k, v := range runtimeVars {
		out[k] = v
	}
	return out
}

func setMacroVariable(m *models.Macro, name string, value any) {
	if m == nil {
		return
	}
	if m.Variables == nil {
		m.Variables = models.NewVariableStore()
	}
	m.Variables.Set(name, value)
	SnapshotRuntimeVariables(m)
}
