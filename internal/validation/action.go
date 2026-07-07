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
	switch n := a.(type) {
	case *actions.Pause:
		return macrohotkey.ValidateContinueKeyForUI(n.ContinueKey)
	case *actions.Key:
		if strings.TrimSpace(n.Key) == "" {
			return fmt.Errorf("key: record a key before saving")
		}
	case *actions.Calculate:
		if strings.TrimSpace(n.Expression) == "" {
			return fmt.Errorf("calculate: expression cannot be empty")
		}
		if v := macro.ValidateCalculateExpression(n.Expression, macroModel); v.BlocksSubmit() {
			return fmt.Errorf("calculate: %s", v.Error)
		}
	case *actions.SetVariable:
		if err := ValidateVariableName(n.VariableName); err != nil {
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
