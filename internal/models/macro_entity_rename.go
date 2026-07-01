package models

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"strings"
)

// ProgramEntityKind identifies a program-owned entity whose name appears in macro actions.
type ProgramEntityKind int

const (
	ProgramEntityPoint ProgramEntityKind = iota
	ProgramEntitySearchArea
	ProgramEntityItem
)

// RenameProgramEntity updates macro actions that reference the renamed entity within program.
// Returns true when any action was modified.
func (m *Macro) RenameProgramEntity(kind ProgramEntityKind, program, oldName, newName string) bool {
	if m == nil || m.Root == nil || oldName == newName {
		return false
	}
	program = strings.TrimSpace(program)
	oldName = strings.TrimSpace(oldName)
	newName = strings.TrimSpace(newName)
	if oldName == "" || newName == "" {
		return false
	}

	changed := false
	walkMacroActions(m.Root, func(a actions.ActionInterface) {
		switch kind {
		case ProgramEntityPoint:
			mv, ok := a.(*actions.Move)
			if !ok {
				return
			}
			if ref := renameCoordinateEntity(mv.Point, program, oldName, newName); ref != mv.Point {
				mv.Point = ref
				changed = true
			}
		case ProgramEntitySearchArea:
			switch n := a.(type) {
			case *actions.ImageSearch:
				if ref := renameCoordinateEntity(n.SearchArea, program, oldName, newName); ref != n.SearchArea {
					n.SearchArea = ref
					changed = true
				}
			case *actions.Ocr:
				if ref := renameCoordinateEntity(n.SearchArea, program, oldName, newName); ref != n.SearchArea {
					n.SearchArea = ref
					changed = true
				}
			case *actions.FindPixel:
				if ref := renameCoordinateEntity(n.SearchArea, program, oldName, newName); ref != n.SearchArea {
					n.SearchArea = ref
					changed = true
				}
			}
		case ProgramEntityItem:
			is, ok := a.(*actions.ImageSearch)
			if !ok || len(is.Targets) == 0 {
				return
			}
			for i, target := range is.Targets {
				if renamed := renameItemTargetEntity(target, program, oldName, newName); renamed != target {
					is.Targets[i] = renamed
					changed = true
				}
			}
		}
	})
	return changed
}

// RenameProgram updates macro actions that reference program-owned entities under the old program name.
// Returns true when any action was modified.
func (m *Macro) RenameProgram(oldProgram, newProgram string) bool {
	if m == nil || m.Root == nil || oldProgram == newProgram {
		return false
	}
	oldProgram = strings.TrimSpace(oldProgram)
	newProgram = strings.TrimSpace(newProgram)
	if oldProgram == "" || newProgram == "" {
		return false
	}

	changed := false
	walkMacroActions(m.Root, func(a actions.ActionInterface) {
		switch n := a.(type) {
		case *actions.Move:
			if ref := renameCoordinateProgram(n.Point, oldProgram, newProgram); ref != n.Point {
				n.Point = ref
				changed = true
			}
		case *actions.ImageSearch:
			if ref := renameCoordinateProgram(n.SearchArea, oldProgram, newProgram); ref != n.SearchArea {
				n.SearchArea = ref
				changed = true
			}
			for i, target := range n.Targets {
				if renamed := renameItemTargetProgram(target, oldProgram, newProgram); renamed != target {
					n.Targets[i] = renamed
					changed = true
				}
			}
		case *actions.Ocr:
			if ref := renameCoordinateProgram(n.SearchArea, oldProgram, newProgram); ref != n.SearchArea {
				n.SearchArea = ref
				changed = true
			}
		case *actions.FindPixel:
			if ref := renameCoordinateProgram(n.SearchArea, oldProgram, newProgram); ref != n.SearchArea {
				n.SearchArea = ref
				changed = true
			}
		}
	})
	return changed
}

// RenameMacroReference updates Run Macro actions that call the renamed macro.
// Returns true when any action was modified.
func (m *Macro) RenameMacroReference(oldName, newName string) bool {
	if m == nil || m.Root == nil || oldName == newName {
		return false
	}
	oldName = strings.TrimSpace(oldName)
	newName = strings.TrimSpace(newName)
	if oldName == "" || newName == "" {
		return false
	}

	changed := false
	walkMacroActions(m.Root, func(a actions.ActionInterface) {
		rm, ok := a.(*actions.RunMacro)
		if !ok || rm.MacroName != oldName {
			return
		}
		rm.MacroName = newName
		changed = true
	})
	return changed
}

func renameCoordinateEntity(ref actions.CoordinateRef, program, oldName, newName string) actions.CoordinateRef {
	if ref.IsEmpty() {
		return ref
	}
	if prog := ref.Program(); prog != "" {
		if prog != program || ref.Name() != oldName {
			return ref
		}
		return actions.NewCoordinateRef(prog, newName)
	}
	if ref.Name() != oldName {
		return ref
	}
	return actions.NewCoordinateRef("", newName)
}

func renameCoordinateProgram(ref actions.CoordinateRef, oldProgram, newProgram string) actions.CoordinateRef {
	if ref.IsEmpty() || ref.Program() != oldProgram {
		return ref
	}
	return actions.NewCoordinateRef(newProgram, ref.Name())
}

func renameItemTargetEntity(target, program, oldItem, newItem string) string {
	prog, rest, ok := strings.Cut(target, config.ProgramDelimiter)
	if !ok || prog != program {
		return target
	}
	base, variant, hasVariant := strings.Cut(rest, config.ProgramDelimiter)
	if base != oldItem {
		return target
	}
	if hasVariant {
		return program + config.ProgramDelimiter + newItem + config.ProgramDelimiter + variant
	}
	return program + config.ProgramDelimiter + newItem
}

func renameItemTargetProgram(target, oldProgram, newProgram string) string {
	prog, rest, ok := strings.Cut(target, config.ProgramDelimiter)
	if !ok || prog != oldProgram {
		return target
	}
	return newProgram + config.ProgramDelimiter + rest
}
