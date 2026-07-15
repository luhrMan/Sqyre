package validation

import (
	"Sqyre/internal/macro"
	"Sqyre/internal/macrohotkey"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"fmt"
	"strings"
)

// ValidateAction checks that an action has the minimum fields required to save and run.
func ValidateAction(a actions.ActionInterface, macroModel *models.Macro) error {
	if a == nil {
		return fmt.Errorf("action cannot be nil")
	}
	if producer, ok := a.(actions.VariableProducer); ok {
		for _, b := range producer.VariableBindings() {
			if strings.TrimSpace(b.Name) == "" {
				continue
			}
			if err := ValidateVariableAssignmentName(b.Name); err != nil {
				return fmt.Errorf("%s: %w", variableBindingLabel(b), err)
			}
		}
	}
	switch n := a.(type) {
	case *actions.Pause:
		return macrohotkey.ValidateContinueKeyForUI(n.ContinueKey)
	case *actions.Key:
		if strings.TrimSpace(n.Key) == "" {
			return fmt.Errorf("key: record a key before saving")
		}
	case *actions.SetVariable:
		if err := ValidateVariableAssignmentName(n.VariableName); err != nil {
			return fmt.Errorf("set variable: %w", err)
		}
		if val, ok := n.Value.(string); ok {
			if v := macro.ValidateSetVariableValue(val, macroModel); v.BlocksSubmit() {
				return fmt.Errorf("set variable: %s", v.Error)
			}
		}
	}
	return nil
}

func variableBindingLabel(b actions.VariableBinding) string {
	name := strings.TrimSpace(b.Name)
	switch b.Role {
	case "value":
		return fmt.Sprintf("variable %q", name)
	case "output":
		return fmt.Sprintf("output variable %q", name)
	case "output_x":
		return fmt.Sprintf("output X variable %q", name)
	case "output_y":
		return fmt.Sprintf("output Y variable %q", name)
	default:
		return fmt.Sprintf("variable %q", name)
	}
}
