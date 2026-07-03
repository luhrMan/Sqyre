package services

import (
	"fmt"
	"maps"
	"sort"
	"sync"

	"Sqyre/internal/models"
)

var (
	runtimeVarsMu sync.RWMutex
	runtimeVars   map[string]string
)

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
}

// GetRuntimeVariables returns the latest snapshot (empty map if none).
func GetRuntimeVariables() map[string]string {
	runtimeVarsMu.RLock()
	defer runtimeVarsMu.RUnlock()
	if runtimeVars == nil {
		return map[string]string{}
	}
	out := make(map[string]string, len(runtimeVars))
	maps.Copy(out, runtimeVars)
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
