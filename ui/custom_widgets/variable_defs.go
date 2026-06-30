package custom_widgets

import (
	"Sqyre/internal/models"
	"strings"
)

// VariableDefLabel formats a variable for picker and completion display.
func VariableDefLabel(d models.VariableDef) string {
	parts := []string{d.Name}
	if d.Type != "" && d.Type != models.VariableTypeAuto {
		parts = append(parts, string(d.Type))
	}
	switch {
	case d.Source.ActionName != "":
		parts = append(parts, d.Source.ActionName)
	case d.Role == models.VariableRoleBuiltin:
		parts = append(parts, "builtin")
	case d.Role == models.VariableRoleOutput || d.Role == models.VariableRoleOutputX || d.Role == models.VariableRoleOutputY:
		parts = append(parts, "output")
	}
	return strings.Join(parts, " · ")
}

func variableDefsFingerprint(defs []models.VariableDef) string {
	if len(defs) == 0 {
		return ""
	}
	var b strings.Builder
	for _, d := range defs {
		b.WriteString(d.Name)
		b.WriteByte('|')
		b.WriteString(string(d.Type))
		b.WriteByte('|')
		b.WriteString(d.Source.ActionUID)
		b.WriteByte(';')
	}
	return b.String()
}

func namesFromDefs(defs []models.VariableDef) []string {
	if len(defs) == 0 {
		return nil
	}
	names := make([]string, len(defs))
	for i, d := range defs {
		names[i] = d.Name
	}
	return names
}

func knownVariableSet(defs []models.VariableDef) map[string]bool {
	known := make(map[string]bool, len(defs))
	for _, d := range defs {
		known[strings.ToLower(strings.TrimSpace(d.Name))] = true
	}
	return known
}
