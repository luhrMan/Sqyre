package models

import (
	"reflect"
	"sort"
	"strconv"
	"strings"

	"Sqyre/internal/models/actions"
	"Sqyre/internal/varref"
)

// VariableUsageKind describes how a variable appears at a location in the macro.
type VariableUsageKind string

const (
	VariableUsageInitial    VariableUsageKind = "initial"
	VariableUsageDefined    VariableUsageKind = "defined"
	VariableUsageReferenced VariableUsageKind = "referenced"
	VariableUsageRead       VariableUsageKind = "read"
)

// VariableUsage is one place a variable appears in a macro.
type VariableUsage struct {
	Kind       VariableUsageKind
	ActionType string
	ActionUID  string
	ActionName string
	Detail     string
	Order      int
}

// CollectVariableUsages returns every location varName appears in the macro:
// declarations, producer bindings, ${var} references, and bare-name reads.
func CollectVariableUsages(m *Macro, varName string) []VariableUsage {
	if m == nil {
		return nil
	}
	varName = strings.TrimSpace(varName)
	if varName == "" {
		return nil
	}

	var out []VariableUsage
	seen := make(map[string]struct{})
	add := func(u VariableUsage) {
		key := u.ActionUID + "\x00" + string(u.Kind) + "\x00" + u.Detail
		if _, ok := seen[key]; ok {
			return
		}
		seen[key] = struct{}{}
		out = append(out, u)
	}

	order := 0
	if decl, ok := m.FindVariableDecl(varName); ok {
		add(VariableUsage{
			Kind:       VariableUsageInitial,
			ActionType: "initial",
			ActionName: "Initial value",
			Detail:     variableUsageInitialDetail(decl),
			Order:      order,
		})
		order++
	}

	if m.Root != nil {
		walkMacroActions(m.Root, func(a actions.ActionInterface) {
			order++
			actionOrder := order
			actionType := a.GetType()
			actionUID := a.GetUID()
			actionName := variableActionLabel(a)

			if producer, ok := a.(actions.VariableProducer); ok {
				for _, b := range producer.VariableBindings() {
					if !variableNameMatches(b.Name, varName) {
						continue
					}
					add(VariableUsage{
						Kind:       VariableUsageDefined,
						ActionType: actionType,
						ActionUID:  actionUID,
						ActionName: actionName,
						Detail:     variableBindingDetail(b),
						Order:      actionOrder,
					})
				}
			}

			if sv, ok := a.(*actions.SaveVariable); ok && variableNameMatches(sv.VariableName, varName) {
				add(VariableUsage{
					Kind:       VariableUsageRead,
					ActionType: actionType,
					ActionUID:  actionUID,
					ActionName: actionName,
					Detail:     "Save to " + strings.TrimSpace(sv.Destination),
					Order:      actionOrder,
				})
			}

			collectVariableRefsInAction(a, varName, func(field string) {
				add(VariableUsage{
					Kind:       VariableUsageReferenced,
					ActionType: actionType,
					ActionUID:  actionUID,
					ActionName: actionName,
					Detail:     field,
					Order:      actionOrder,
				})
			})
		})
	}

	sort.Slice(out, func(i, j int) bool { return out[i].Order < out[j].Order })
	return out
}

func variableUsageInitialDetail(decl VariableDecl) string {
	parts := []string{"Declared in Variables panel"}
	if v := strings.TrimSpace(decl.InitialValue); v != "" {
		parts = append(parts, "initial: "+v)
	}
	if t := decl.Type; t != "" && t != VariableTypeAuto {
		parts = append(parts, "type: "+string(t))
	}
	return strings.Join(parts, " · ")
}

func variableBindingDetail(b actions.VariableBinding) string {
	switch b.Role {
	case "value":
		return "Set variable"
	case "output":
		if b.Conditional {
			return "Output (conditional)"
		}
		return "Output"
	case "output_x":
		if b.Conditional {
			return "Output X (conditional)"
		}
		return "Output X"
	case "output_y":
		if b.Conditional {
			return "Output Y (conditional)"
		}
		return "Output Y"
	case "length":
		return "Length output"
	default:
		return "Defined"
	}
}

func variableNameMatches(have, want string) bool {
	return strings.EqualFold(strings.TrimSpace(have), strings.TrimSpace(want))
}

func collectVariableRefsInAction(a actions.ActionInterface, varName string, onField func(field string)) {
	v := reflect.ValueOf(a)
	if v.Kind() != reflect.Pointer || v.IsNil() {
		return
	}
	collectRefsInValue(v.Elem(), varName, "", onField)
}

func collectRefsInValue(v reflect.Value, varName, fieldPath string, onField func(field string)) {
	switch v.Kind() {
	case reflect.String:
		if varref.References(v.String(), varName) {
			onField(fieldPath)
		}
	case reflect.Interface:
		if v.IsNil() {
			return
		}
		el := v.Elem()
		if el.Kind() == reflect.String && varref.References(el.String(), varName) {
			onField(fieldPath)
		}
	case reflect.Struct:
		t := v.Type()
		for i := 0; i < v.NumField(); i++ {
			f := t.Field(i)
			if f.PkgPath != "" {
				continue
			}
			ft := f.Type
			if ft.Kind() == reflect.Pointer {
				continue
			}
			if ft.Kind() == reflect.Interface && (ft == actionIfaceType || ft == advIfaceType) {
				continue
			}
			if ft.Kind() == reflect.Slice && ft.Elem() == actionIfaceType {
				continue
			}
			path := f.Name
			if fieldPath != "" {
				path = fieldPath + " · " + f.Name
			}
			collectRefsInValue(v.Field(i), varName, path, onField)
		}
	case reflect.Slice, reflect.Array:
		if v.Type().Elem() == actionIfaceType {
			return
		}
		for i := 0; i < v.Len(); i++ {
			idxPath := fieldPath
			if v.Len() > 1 {
				idxPath = fieldPath + " · " + strconv.Itoa(i+1)
			}
			collectRefsInValue(v.Index(i), varName, idxPath, onField)
		}
	}
}
