package models

import (
	"Sqyre/internal/models/actions"
	"Sqyre/internal/screen"
	"sort"
	"strings"
)

// CollectVariableDefs walks the macro and returns variable definitions with source metadata.
func CollectVariableDefs(m *Macro) []VariableDef {
	if m == nil {
		return nil
	}
	seen := make(map[string]VariableDef) // keyed by strings.ToLower(name)
	hasImageSearch := false
	hasForEachRow := false

	addDef := func(name string, def VariableDef) {
		key := strings.ToLower(name)
		if existing, ok := seen[key]; ok {
			merged := existing
			merged.Name = preferVariableName(existing.Name, name)
			if def.InitialValue != "" {
				merged.InitialValue = def.InitialValue
			}
			if def.Type != "" {
				merged.Type = def.Type
			}
			if def.Description != "" {
				merged.Description = def.Description
			}
			if existing.Source.ActionType == "initial" && def.Source.ActionType != "" {
				merged.Source = def.Source
				merged.Role = def.Role
			}
			seen[key] = merged
			return
		}
		def.Name = name
		seen[key] = def
	}

	if m.Root != nil {
		walkMacroActions(m.Root, func(a actions.ActionInterface) {
			if _, ok := a.(*actions.ImageSearch); ok {
				hasImageSearch = true
			}
			if _, ok := a.(*actions.ForEachRow); ok {
				hasForEachRow = true
			}
			producer, ok := a.(actions.VariableProducer)
			if !ok {
				return
			}
			for _, b := range producer.VariableBindings() {
				name := strings.TrimSpace(b.Name)
				if name == "" {
					continue
				}
				addDef(name, VariableDef{
					Role: variableRoleFromBinding(b.Role),
					Source: VariableSource{
						ActionType:  a.GetType(),
						ActionUID:   a.GetUID(),
						ActionName:  variableActionLabel(a),
						Conditional: b.Conditional,
					},
				})
			}
		})
	}

	if hasImageSearch {
		for _, name := range ImageSearchBuiltinVars {
			addDef(name, VariableDef{
				Role: VariableRoleBuiltin,
				Source: VariableSource{
					ActionType:  "imagesearch",
					ActionName:  "Image Search (sub-action)",
					Conditional: true,
				},
			})
		}
	}

	if hasForEachRow {
		for _, name := range ForEachRowBuiltinVars {
			addDef(name, VariableDef{
				Role: VariableRoleBuiltin,
				Source: VariableSource{
					ActionType:  "foreachrow",
					ActionName:  "For each row (sub-action)",
					Conditional: true,
				},
			})
		}
	}

	for _, name := range MonitorBuiltinVarNames(screen.NumDisplays()) {
		addDef(name, VariableDef{
			Role: VariableRoleBuiltin,
			Source: VariableSource{
				ActionType: "system",
				ActionName: "Monitor size",
			},
		})
	}

	for _, decl := range m.VariableDecls {
		name := strings.TrimSpace(decl.Name)
		if name == "" {
			continue
		}
		typ := decl.Type
		if typ == "" {
			typ = VariableTypeAuto
		}
		addDef(name, VariableDef{
			Role:         VariableRoleValue,
			Type:         typ,
			Description:  decl.Description,
			InitialValue: decl.InitialValue,
			Source: VariableSource{
				ActionType: "initial",
				ActionName: "Initial value",
			},
		})
	}

	defs := make([]VariableDef, 0, len(seen))
	for _, d := range seen {
		defs = append(defs, d)
	}
	sort.Slice(defs, func(i, j int) bool { return defs[i].Name < defs[j].Name })
	return defs
}

// CollectDefinedVariables returns sorted variable names for completion menus.
func CollectDefinedVariableNames(m *Macro) []string {
	defs := CollectVariableDefs(m)
	if len(defs) == 0 {
		return nil
	}
	names := make([]string, len(defs))
	for i, d := range defs {
		names[i] = d.Name
	}
	return names
}

// WalkActions visits every action in an action tree, including nested sub-actions.
func WalkActions(a actions.ActionInterface, visit func(actions.ActionInterface)) {
	walkMacroActions(a, visit)
}

func walkMacroActions(a actions.ActionInterface, visit func(actions.ActionInterface)) {
	if a == nil {
		return
	}
	visit(a)
	if adv, ok := a.(actions.AdvancedActionInterface); ok {
		for _, sub := range adv.GetSubActions() {
			walkMacroActions(sub, visit)
		}
	}
}

func variableRoleFromBinding(role string) VariableRole {
	switch role {
	case "output":
		return VariableRoleOutput
	case "output_x":
		return VariableRoleOutputX
	case "output_y":
		return VariableRoleOutputY
	case "length":
		return VariableRoleLength
	case "builtin":
		return VariableRoleBuiltin
	default:
		return VariableRoleValue
	}
}

func variableActionLabel(a actions.ActionInterface) string {
	switch n := a.(type) {
	case *actions.Loop:
		if n.Name != "" && n.Name != "root" {
			return n.Name
		}
	case *actions.ImageSearch:
		if n.Name != "" {
			return n.Name
		}
	case *actions.Ocr:
		if n.Name != "" {
			return n.Name
		}
	case *actions.FindPixel:
		if n.Name != "" {
			return n.Name
		}
	case *actions.ForEachRow:
		if n.Name != "" {
			return n.Name
		}
	}
	return a.GetType()
}
