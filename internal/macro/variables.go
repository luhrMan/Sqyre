package macro

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/varref"
	"fmt"
	"math"
	"strconv"
	"strings"
)

// Variable reference grammar (${VarName} / {VarName}) lives in internal/varref.
// The substitution engine below sources the patterns from there.
var (
	varPattern    = varref.DollarPattern
	varPatternAlt = varref.BracePattern
)

// ResolveVariables resolves variable references in a string.
// Supports ${VarName} and {VarName} syntax.
// Item mask shape fields also resolve ${StackMax}, ${Cols}, ${Rows}, ${ItemName}, ${ImagePixelWidth}, ${ImagePixelHeight} from the template being matched.
// Image Search sub-actions get the same item variables after each match.
// For Each Row sub-actions also get ${Row} (1-based) and ${RowCount}.
// Every macro run also sets monitor builtins: ${monitor1Width}, ${monitor1Height}, ${monitor2Width}, ...
func ResolveVariables(text string, macro *models.Macro) (string, error) {
	if macro == nil || macro.Variables == nil {
		return text, nil
	}

	result := text

	// Replace ${VarName} patterns
	result = varPattern.ReplaceAllStringFunc(result, func(match string) string {
		varName := strings.TrimSpace(varPattern.FindStringSubmatch(match)[1])
		if val, ok := macro.Variables.Get(varName); ok {
			return fmt.Sprintf("%v", val)
		}
		return match // Return original if variable not found
	})

	// Replace {VarName} patterns (only if not already replaced)
	result = varPatternAlt.ReplaceAllStringFunc(result, func(match string) string {
		// Skip if this looks like it was already processed
		if strings.HasPrefix(match, "${") {
			return match
		}
		varName := strings.TrimSpace(varPatternAlt.FindStringSubmatch(match)[1])
		if val, ok := macro.Variables.Get(varName); ok {
			return fmt.Sprintf("%v", val)
		}
		return match
	})

	return result, nil
}

// EvaluateCondition evaluates a Conditional action's clauses using its match
// mode (all = AND, any = OR). An empty clause list is treated as false.
func EvaluateCondition(node *actions.Conditional, macro *models.Macro) (bool, error) {
	if len(node.Clauses) == 0 {
		return false, nil
	}
	if node.EffectiveMatch() == actions.MatchAny {
		for i, clause := range node.Clauses {
			ok, err := evaluateClause(clause, macro)
			if err != nil {
				return false, fmt.Errorf("clause %d: %w", i+1, err)
			}
			if ok {
				return true, nil
			}
		}
		return false, nil
	}
	for i, clause := range node.Clauses {
		ok, err := evaluateClause(clause, macro)
		if err != nil {
			return false, fmt.Errorf("clause %d: %w", i+1, err)
		}
		if !ok {
			return false, nil
		}
	}
	return true, nil
}

// evaluateClause resolves operands and evaluates one comparison. Numeric operands
// are compared numerically; otherwise comparison falls back to string semantics.
// Unary operators (is set / is empty) inspect only the left operand.
func evaluateClause(clause actions.ConditionClause, macro *models.Macro) (bool, error) {
	left, err := resolveOperand(clause.Left, macro)
	if err != nil {
		return false, fmt.Errorf("left operand: %w", err)
	}

	switch clause.Operator {
	case actions.OpIsSet:
		return left != "" && checkUnresolvedVariable(left) == nil, nil
	case actions.OpIsEmpty:
		return left == "" || checkUnresolvedVariable(left) != nil, nil
	}

	right, err := resolveOperand(clause.Right, macro)
	if err != nil {
		return false, fmt.Errorf("right operand: %w", err)
	}

	switch clause.Operator {
	case actions.OpContains:
		return strings.Contains(left, right), nil
	case actions.OpStartsWith:
		return strings.HasPrefix(left, right), nil
	case actions.OpEndsWith:
		return strings.HasSuffix(left, right), nil
	}

	// Numeric comparison when both operands parse as numbers, otherwise string.
	lf, lok := parseConditionNumber(left)
	rf, rok := parseConditionNumber(right)
	numeric := lok && rok

	switch clause.Operator {
	case actions.OpEquals:
		if numeric {
			return lf == rf, nil
		}
		return left == right, nil
	case actions.OpNotEquals:
		if numeric {
			return lf != rf, nil
		}
		return left != right, nil
	case actions.OpLess:
		if numeric {
			return lf < rf, nil
		}
		return left < right, nil
	case actions.OpLessEqual:
		if numeric {
			return lf <= rf, nil
		}
		return left <= right, nil
	case actions.OpGreater:
		if numeric {
			return lf > rf, nil
		}
		return left > right, nil
	case actions.OpGreaterEq:
		if numeric {
			return lf >= rf, nil
		}
		return left >= right, nil
	default:
		return false, fmt.Errorf("unknown operator %q", clause.Operator)
	}
}

// resolveOperand resolves variable references in an operand to a string.
func resolveOperand(value any, macro *models.Macro) (string, error) {
	switch v := value.(type) {
	case nil:
		return "", nil
	case string:
		return ResolveVariables(v, macro)
	default:
		return fmt.Sprintf("%v", v), nil
	}
}

// parseConditionNumber reports whether s parses as a number and returns its value.
func parseConditionNumber(s string) (float64, bool) {
	f, err := strconv.ParseFloat(strings.TrimSpace(s), 64)
	if err != nil {
		return 0, false
	}
	return f, true
}

// EntryValidation is the outcome of validating a variable entry value in the UI.
// Warnings (e.g. undefined ${variable}) are shown but do not block submission.
// Errors (e.g. malformed expressions) block submission.
type EntryValidation struct {
	Warning string
	Error   string
}

// BlocksSubmit reports whether the entry should prevent saving.
func (v EntryValidation) BlocksSubmit() bool {
	return v.Error != ""
}

// UnknownVariableWarning returns a warning when text references undefined variables.
func UnknownVariableWarning(text string, macro *models.Macro) string {
	if strings.TrimSpace(text) == "" || macro == nil {
		return ""
	}

	known := make(map[string]bool)
	for _, n := range macro.CollectDefinedVariables() {
		known[strings.ToLower(strings.TrimSpace(n))] = true
	}

	var unknown []string
	for _, r := range varref.Names(text) {
		name := strings.TrimSpace(r)
		if name == "" {
			continue
		}
		if !known[strings.ToLower(name)] {
			unknown = append(unknown, name)
		}
	}
	if len(unknown) == 1 {
		return fmt.Sprintf("unknown variable %q", unknown[0])
	}
	if len(unknown) > 1 {
		return fmt.Sprintf("unknown variables: %s", strings.Join(unknown, ", "))
	}
	return ""
}

// validateExpressionStructure checks that an expression parses and evaluates when
// every referenced variable is seeded with a numeric placeholder.
func validateExpressionStructure(expr string, macro *models.Macro) error {
	if strings.TrimSpace(expr) == "" || macro == nil {
		return nil
	}

	macro.InitRuntimeVariables()
	for _, r := range varref.Names(expr) {
		name := strings.TrimSpace(r)
		if name == "" {
			continue
		}
		if _, ok := macro.Variables.Get(name); !ok {
			macro.Variables.Set(name, 0)
		}
	}
	_, err := EvaluateExpression(expr, macro)
	return err
}

// LooksLikeExpression reports whether text will be evaluated as arithmetic at runtime.
func LooksLikeExpression(text string) bool {
	t := strings.TrimSpace(text)
	if t == "" {
		return false
	}
	if strings.ContainsAny(t, "+*/^()") {
		return true
	}
	for i := 0; i < len(t); i++ {
		if t[i] != '-' {
			continue
		}
		if i == 0 {
			return true
		}
		prev, next := t[i-1], byte(0)
		if i+1 < len(t) {
			next = t[i+1]
		}
		if isExprNumberChar(prev) || isExprNumberChar(next) || prev == ')' || next == '(' {
			return true
		}
	}
	lower := strings.ToLower(t)
	for _, fn := range []string{"sqrt", "abs", "round", "floor", "ceil", "trunc", "sin", "cos", "tan", "ln"} {
		if strings.Contains(lower, fn+"(") {
			return true
		}
	}
	return strings.Contains(t, "~pi") || strings.Contains(t, "~e")
}

func isExprNumberChar(b byte) bool {
	return b >= '0' && b <= '9' || b == '.'
}

// ValidateVariableReferences returns a warning-only validation for ${variable} references.
func ValidateVariableReferences(text string, macro *models.Macro) EntryValidation {
	return EntryValidation{Warning: UnknownVariableWarning(text, macro)}
}

// ValidateNumericExpression checks that text is empty, a literal number, or a valid
// arithmetic expression. Unknown variables produce a warning but do not block.
func ValidateNumericExpression(text string, macro *models.Macro) EntryValidation {
	if strings.TrimSpace(text) == "" {
		return EntryValidation{}
	}
	v := EntryValidation{Warning: UnknownVariableWarning(text, macro)}
	if err := validateExpressionStructure(text, macro); err != nil {
		v.Error = err.Error()
	}
	return v
}

// ValidateSetVariableValue checks set-variable values: plain text is allowed.
// Unknown variables warn; invalid arithmetic blocks.
func ValidateSetVariableValue(text string, macro *models.Macro) EntryValidation {
	if strings.TrimSpace(text) == "" {
		return EntryValidation{}
	}
	v := EntryValidation{Warning: UnknownVariableWarning(text, macro)}
	if LooksLikeExpression(text) {
		if err := validateExpressionStructure(text, macro); err != nil {
			v.Error = err.Error()
		}
	}
	return v
}

// PreviewCalculate validates and evaluates an arithmetic expression for the editor
// preview (Set values that look like expressions). It distinguishes these cases:
//   - undefined ${variable} references -> preview still runs (warning shown in the entry field)
//   - all referenced variables are defined and have current values -> "= <result>"
//   - referenced variables without current values yet -> "valid (result depends on runtime values)"
//
// An empty expression returns ("", nil).
func PreviewCalculate(expr string, macro *models.Macro) (string, error) {
	if strings.TrimSpace(expr) == "" || macro == nil {
		return "", nil
	}

	macro.InitRuntimeVariables()

	refs := varref.Names(expr)

	// Seed every referenced variable with a numeric placeholder so structurally valid
	// expressions evaluate even when names are not declared yet.
	runtimeDependent := false
	for _, r := range refs {
		name := strings.TrimSpace(r)
		if name == "" {
			continue
		}
		if _, ok := macro.Variables.Get(name); !ok {
			macro.Variables.Set(name, 0)
			runtimeDependent = true
		}
	}

	res, err := EvaluateExpression(expr, macro)
	if err != nil {
		return "", err
	}

	if runtimeDependent || UnknownVariableWarning(expr, macro) != "" {
		return "valid (result depends on runtime values)", nil
	}
	if f, ok := res.(float64); ok {
		return "= " + strconv.FormatFloat(f, 'g', -1, 64), nil
	}
	return "= " + fmt.Sprintf("%v", res), nil
}

// EvaluateExpression evaluates a mathematical expression with variable substitution
// Supports: +, -, *, /, ^, functions (sqrt, abs, round, floor, ceil, trunc, sin, cos, tan, ln), constants (~pi, ~e)
func EvaluateExpression(expr string, macro *models.Macro) (any, error) {
	resolved, err := ResolveVariables(expr, macro)
	if err != nil {
		return nil, err
	}

	resolved = strings.ReplaceAll(resolved, "~pi", fmt.Sprintf("%f", math.Pi))
	resolved = strings.ReplaceAll(resolved, "~e", fmt.Sprintf("%f", math.E))

	result, err := evaluateNumericExpression(resolved)
	if err != nil {
		return nil, fmt.Errorf("failed to evaluate expression: %w", err)
	}

	return result, nil
}

func lookupVariable(name string, macro *models.Macro, overrides map[string]any) (any, bool) {
	name = strings.TrimSpace(name)
	if overrides != nil {
		if v, ok := overrides[name]; ok {
			return v, true
		}
		lower := strings.ToLower(name)
		for k, v := range overrides {
			if strings.ToLower(k) == lower {
				return v, true
			}
		}
	}
	if macro != nil && macro.Variables != nil {
		return macro.Variables.Get(name)
	}
	return nil, false
}

func resolveVariablesWithOverrides(text string, macro *models.Macro, overrides map[string]any) (string, error) {
	if overrides == nil && (macro == nil || macro.Variables == nil) {
		return text, nil
	}

	result := text
	result = varPattern.ReplaceAllStringFunc(result, func(match string) string {
		varName := strings.TrimSpace(varPattern.FindStringSubmatch(match)[1])
		if val, ok := lookupVariable(varName, macro, overrides); ok {
			return fmt.Sprintf("%v", val)
		}
		return match
	})

	result = varPatternAlt.ReplaceAllStringFunc(result, func(match string) string {
		if strings.HasPrefix(match, "${") {
			return match
		}
		varName := strings.TrimSpace(varPatternAlt.FindStringSubmatch(match)[1])
		if val, ok := lookupVariable(varName, macro, overrides); ok {
			return fmt.Sprintf("%v", val)
		}
		return match
	})

	return result, nil
}

func evaluateExpressionWithOverrides(expr string, macro *models.Macro, overrides map[string]any) (any, error) {
	resolved, err := resolveVariablesWithOverrides(expr, macro, overrides)
	if err != nil {
		return nil, err
	}
	resolved = strings.ReplaceAll(resolved, "~pi", fmt.Sprintf("%f", math.Pi))
	resolved = strings.ReplaceAll(resolved, "~e", fmt.Sprintf("%f", math.E))
	result, err := evaluateNumericExpression(resolved)
	if err != nil {
		return nil, fmt.Errorf("failed to evaluate expression: %w", err)
	}
	return result, nil
}

func ResolveIntWithOverrides(value any, macro *models.Macro, overrides map[string]any) (int, error) {
	switch v := value.(type) {
	case int:
		return v, nil
	case float64:
		return int(v), nil
	case string:
		resolved, err := resolveVariablesWithOverrides(v, macro, overrides)
		if err != nil {
			return 0, err
		}
		if err := checkUnresolvedVariable(resolved); err != nil {
			return 0, err
		}
		if strings.ContainsAny(resolved, "+-*/^()") {
			result, err := evaluateExpressionWithOverrides(resolved, macro, overrides)
			if err != nil {
				return 0, err
			}
			if f, ok := result.(float64); ok {
				return int(f), nil
			}
		}
		val, err := strconv.Atoi(resolved)
		if err != nil {
			return 0, fmt.Errorf("cannot convert %s to int: %w", resolved, err)
		}
		return val, nil
	default:
		return 0, fmt.Errorf("unsupported type for int resolution: %T", value)
	}
}

// checkUnresolvedVariable returns an error if s still contains an unresolved variable reference.
func checkUnresolvedVariable(s string) error {
	if strings.Contains(s, "${") {
		if sub := varPattern.FindStringSubmatch(s); len(sub) > 1 {
			return fmt.Errorf("variable %q not defined (set by an earlier Image Search output or macro variables; check execution order and matching names)", sub[1])
		}
		return fmt.Errorf("unresolved variable reference: %s", s)
	}
	if sub := varPatternAlt.FindStringSubmatch(s); len(sub) > 1 && !strings.HasPrefix(s, "${") {
		return fmt.Errorf("variable %q not defined (set by an earlier Image Search output or macro variables; check execution order and matching names)", sub[1])
	}
	return nil
}

// ResolveInt resolves a variable reference or expression to an integer
func ResolveInt(value any, macro *models.Macro) (int, error) {
	switch v := value.(type) {
	case int:
		return v, nil
	case float64:
		return int(v), nil
	case string:
		// Try to resolve as variable or expression
		resolved, err := ResolveVariables(v, macro)
		if err != nil {
			return 0, err
		}
		// Unresolved variable reference (e.g. variable not defined)
		if err := checkUnresolvedVariable(resolved); err != nil {
			return 0, err
		}
		// Try to evaluate as expression (only if it looks like one: has operators)
		if strings.ContainsAny(resolved, "+-*/^()") {
			result, err := EvaluateExpression(resolved, macro)
			if err != nil {
				return 0, err
			}
			if f, ok := result.(float64); ok {
				return int(f), nil
			}
		}
		// Otherwise parse as number
		val, err := strconv.Atoi(resolved)
		if err != nil {
			return 0, fmt.Errorf("cannot convert %s to int: %w", resolved, err)
		}
		return val, nil
	default:
		return 0, fmt.Errorf("unsupported type for int resolution: %T", value)
	}
}

// ResolveFloat resolves a variable reference or expression to a float
func ResolveFloat(value any, macro *models.Macro) (float64, error) {
	switch v := value.(type) {
	case float64:
		return v, nil
	case int:
		return float64(v), nil
	case string:
		// Try to resolve as variable or expression
		resolved, err := ResolveVariables(v, macro)
		if err != nil {
			return 0, err
		}
		// Unresolved variable reference (e.g. variable not defined)
		if err := checkUnresolvedVariable(resolved); err != nil {
			return 0, err
		}
		// Try to evaluate as expression (only if it looks like one: has operators)
		if strings.ContainsAny(resolved, "+-*/^()") {
			result, err := EvaluateExpression(resolved, macro)
			if err != nil {
				return 0, err
			}
			if f, ok := result.(float64); ok {
				return f, nil
			}
		}
		// Otherwise parse as number
		val, err := strconv.ParseFloat(resolved, 64)
		if err != nil {
			return 0, fmt.Errorf("cannot convert %s to float: %w", resolved, err)
		}
		return val, nil
	default:
		return 0, fmt.Errorf("unsupported type for float resolution: %T", value)
	}
}

// ResolveSetVariableValue resolves ${references} in a Set Variable value and parses numbers when possible.
func ResolveSetVariableValue(value any, macro *models.Macro) (any, error) {
	switch v := value.(type) {
	case int, int64, float32, float64, bool:
		return v, nil
	case string:
		resolved, err := ResolveVariables(v, macro)
		if err != nil {
			return nil, err
		}
		if err := checkUnresolvedVariable(resolved); err != nil {
			return nil, err
		}
		if resolved == "" {
			return "", nil
		}
		if LooksLikeExpression(resolved) {
			if result, err := EvaluateExpression(v, macro); err == nil {
				return result, nil
			}
		}
		if i, err := strconv.Atoi(resolved); err == nil {
			return i, nil
		}
		if f, err := strconv.ParseFloat(resolved, 64); err == nil {
			return f, nil
		}
		return resolved, nil
	default:
		return fmt.Sprintf("%v", v), nil
	}
}

// ResolveString resolves a variable reference to a string
func ResolveString(value any, macro *models.Macro) (string, error) {
	switch v := value.(type) {
	case string:
		return ResolveVariables(v, macro)
	default:
		return fmt.Sprintf("%v", v), nil
	}
}

// ResolveSearchAreaCoords resolves LeftX, TopY, RightX, BottomY (variable refs or expressions) to ints.
// Returns (leftX, topY, rightX, bottomY, error). Used by image search and OCR at runtime.
func ResolveSearchAreaCoords(leftX, topY, rightX, bottomY any, macro *models.Macro) (int, int, int, int, error) {
	lx, err := ResolveInt(leftX, macro)
	if err != nil {
		return 0, 0, 0, 0, fmt.Errorf("LeftX: %w", err)
	}
	ty, err := ResolveInt(topY, macro)
	if err != nil {
		return 0, 0, 0, 0, fmt.Errorf("TopY: %w", err)
	}
	rx, err := ResolveInt(rightX, macro)
	if err != nil {
		return 0, 0, 0, 0, fmt.Errorf("RightX: %w", err)
	}
	by, err := ResolveInt(bottomY, macro)
	if err != nil {
		return 0, 0, 0, 0, fmt.Errorf("BottomY: %w", err)
	}
	return lx, ty, rx, by, nil
}
