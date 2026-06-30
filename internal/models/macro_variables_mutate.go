package models

import (
	"fmt"
	"reflect"
	"regexp"
	"strings"

	"Sqyre/internal/models/actions"
)

// UpsertVariable adds or updates a declared variable by name (case-insensitive).
func (m *Macro) UpsertVariable(decl VariableDecl) {
	decl.Name = strings.TrimSpace(decl.Name)
	if decl.Name == "" {
		return
	}
	if decl.Type == "" {
		decl.Type = VariableTypeAuto
	}
	for i := range m.VariableDecls {
		if strings.EqualFold(m.VariableDecls[i].Name, decl.Name) {
			m.VariableDecls[i] = decl
			return
		}
	}
	m.VariableDecls = append(m.VariableDecls, decl)
}

// RemoveVariable deletes a declared variable by name (case-insensitive).
func (m *Macro) RemoveVariable(name string) {
	name = strings.TrimSpace(name)
	if name == "" {
		return
	}
	out := make([]VariableDecl, 0, len(m.VariableDecls))
	for _, d := range m.VariableDecls {
		if strings.EqualFold(d.Name, name) {
			continue
		}
		out = append(out, d)
	}
	m.VariableDecls = out
}

// FindVariableDecl returns the declaration matching name (case-insensitive).
func (m *Macro) FindVariableDecl(name string) (VariableDecl, bool) {
	name = strings.TrimSpace(name)
	for _, d := range m.VariableDecls {
		if strings.EqualFold(d.Name, name) {
			return d, true
		}
	}
	return VariableDecl{}, false
}

// RenameVariable renames a variable everywhere it is used: the declaration (if
// any), producing-action output bindings, and ${old}/{old} references across the
// action tree. Returns an error for empty names or when newName collides with a
// different existing declaration.
func (m *Macro) RenameVariable(oldName, newName string) error {
	oldName = strings.TrimSpace(oldName)
	newName = strings.TrimSpace(newName)
	if oldName == "" || newName == "" {
		return fmt.Errorf("variable name cannot be empty")
	}
	if oldName == newName {
		return nil
	}
	if !strings.EqualFold(oldName, newName) {
		if _, exists := m.FindVariableDecl(newName); exists {
			return fmt.Errorf("a variable named %q already exists", newName)
		}
	}

	for i := range m.VariableDecls {
		if strings.EqualFold(m.VariableDecls[i].Name, oldName) {
			m.VariableDecls[i].Name = newName
		}
	}

	if m.Root != nil {
		walkMacroActions(m.Root, func(a actions.ActionInterface) {
			renameActionBinding(a, oldName, newName)
			rewriteVariableRefsInAction(a, oldName, newName)
		})
	}
	return nil
}

// renameActionBinding updates the bare output-variable name fields of
// variable-producing actions when they match oldName (case-insensitive).
func renameActionBinding(a actions.ActionInterface, oldName, newName string) {
	switch n := a.(type) {
	case *actions.Calculate:
		if strings.EqualFold(n.OutputVar, oldName) {
			n.OutputVar = newName
		}
	case *actions.SetVariable:
		if strings.EqualFold(n.VariableName, oldName) {
			n.VariableName = newName
		}
	case *actions.ImageSearch:
		if strings.EqualFold(n.OutputXVariable, oldName) {
			n.OutputXVariable = newName
		}
		if strings.EqualFold(n.OutputYVariable, oldName) {
			n.OutputYVariable = newName
		}
	case *actions.Ocr:
		if strings.EqualFold(n.OutputVariable, oldName) {
			n.OutputVariable = newName
		}
		if strings.EqualFold(n.OutputXVariable, oldName) {
			n.OutputXVariable = newName
		}
		if strings.EqualFold(n.OutputYVariable, oldName) {
			n.OutputYVariable = newName
		}
	case *actions.FindPixel:
		if strings.EqualFold(n.OutputXVariable, oldName) {
			n.OutputXVariable = newName
		}
		if strings.EqualFold(n.OutputYVariable, oldName) {
			n.OutputYVariable = newName
		}
	case *actions.ForEachRow:
		for i := range n.Sources {
			if strings.EqualFold(n.Sources[i].OutputVar, oldName) {
				n.Sources[i].OutputVar = newName
			}
		}
	case *actions.SaveVariable:
		if strings.EqualFold(n.VariableName, oldName) {
			n.VariableName = newName
		}
	}
}

var (
	actionIfaceType = reflect.TypeOf((*actions.ActionInterface)(nil)).Elem()
	advIfaceType    = reflect.TypeOf((*actions.AdvancedActionInterface)(nil)).Elem()
)

// rewriteVariableRefsInAction rewrites ${old}/{old} references in the action's
// own string and interface(string) fields. It does not recurse into sub-actions
// or the parent pointer; the caller walks the tree separately.
func rewriteVariableRefsInAction(a actions.ActionInterface, oldName, newName string) {
	v := reflect.ValueOf(a)
	if v.Kind() != reflect.Ptr || v.IsNil() {
		return
	}
	rewriteRefsInValue(v.Elem(), oldName, newName)
}

func rewriteRefsInValue(v reflect.Value, oldName, newName string) {
	switch v.Kind() {
	case reflect.String:
		if v.CanSet() {
			v.SetString(renameRefInString(v.String(), oldName, newName))
		}
	case reflect.Interface:
		if v.IsNil() || !v.CanSet() {
			return
		}
		el := v.Elem()
		if el.Kind() == reflect.String {
			v.Set(reflect.ValueOf(renameRefInString(el.String(), oldName, newName)))
		}
	case reflect.Struct:
		t := v.Type()
		for i := 0; i < v.NumField(); i++ {
			f := t.Field(i)
			if f.PkgPath != "" { // unexported
				continue
			}
			ft := f.Type
			if ft.Kind() == reflect.Ptr {
				continue // skip embedded base actions and the parent pointer
			}
			if ft.Kind() == reflect.Interface && (ft == actionIfaceType || ft == advIfaceType) {
				continue
			}
			if ft.Kind() == reflect.Slice && ft.Elem() == actionIfaceType {
				continue // sub-actions are visited by the tree walk
			}
			rewriteRefsInValue(v.Field(i), oldName, newName)
		}
	case reflect.Slice, reflect.Array:
		if v.Type().Elem() == actionIfaceType {
			return
		}
		for i := 0; i < v.Len(); i++ {
			rewriteRefsInValue(v.Index(i), oldName, newName)
		}
	}
}

// renameRefInString replaces ${old} and {old} references (case-insensitive,
// tolerating surrounding spaces) with the new name, preserving the brace style.
func renameRefInString(s, oldName, newName string) string {
	if s == "" {
		return s
	}
	quoted := regexp.QuoteMeta(oldName)
	// "$$" emits a literal "$" in regexp replacement strings.
	dollar := regexp.MustCompile(`(?i)\$\{\s*` + quoted + `\s*\}`)
	s = dollar.ReplaceAllString(s, "$${"+newName+"}")
	brace := regexp.MustCompile(`(?i)(^|[^$])\{\s*` + quoted + `\s*\}`)
	s = brace.ReplaceAllString(s, "${1}{"+newName+"}")
	return s
}
