package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
)

var (
	// Variable reference patterns: ${VarName} or {VarName}
	varPattern     = regexp.MustCompile(`\$\{([^}]+)\}`)
	varPatternAlt  = regexp.MustCompile(`\{([^}]+)\}`)
	expressionExpr = regexp.MustCompile(`^[0-9+\-*/().\s\w]+$`)
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

// ParseVariableReference extracts variable names from text
func ParseVariableReference(text string) []string {
	varNames := make(map[string]bool)

	// Find ${VarName} patterns
	matches := varPattern.FindAllStringSubmatch(text, -1)
	for _, match := range matches {
		if len(match) > 1 {
			varNames[match[1]] = true
		}
	}

	// Find {VarName} patterns
	matches = varPatternAlt.FindAllStringSubmatch(text, -1)
	for _, match := range matches {
		if len(match) > 1 {
			varNames[match[1]] = true
		}
	}

	result := make([]string, 0, len(varNames))
	for name := range varNames {
		result = append(result, name)
	}
	return result
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
	for _, r := range ParseVariableReference(text) {
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
	for _, r := range ParseVariableReference(expr) {
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
	if strings.ContainsAny(t, "+-*/^()") {
		return true
	}
	lower := strings.ToLower(t)
	for _, fn := range []string{"sqrt", "abs", "round", "floor", "ceil", "trunc", "sin", "cos", "tan", "ln"} {
		if strings.Contains(lower, fn+"(") {
			return true
		}
	}
	return strings.Contains(t, "~pi") || strings.Contains(t, "~e")
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

// ValidateCalculateExpression checks a Calculate action expression.
func ValidateCalculateExpression(text string, macro *models.Macro) EntryValidation {
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

// PreviewCalculate validates and evaluates a Calculate expression for the editor
// preview. It distinguishes these cases:
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

	refs := ParseVariableReference(expr)

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
	// First resolve variables in the expression
	resolved, err := ResolveVariables(expr, macro)
	if err != nil {
		return nil, err
	}

	// Replace escaped constants (~pi, ~e)
	resolved = strings.ReplaceAll(resolved, "~pi", fmt.Sprintf("%f", math.Pi))
	resolved = strings.ReplaceAll(resolved, "~e", fmt.Sprintf("%f", math.E))

	// Replace function names with their values
	resolved = replaceFunctions(resolved)
	result, err := evaluateSimpleExpression(resolved)
	if err != nil {
		return nil, fmt.Errorf("failed to evaluate expression: %w", err)
	}

	return result, nil
}

// replaceFunctions replaces function calls with their results
func replaceFunctions(expr string) string {
	// This is a simplified version - in production, use a proper parser
	// For now, we'll handle basic cases
	expr = regexp.MustCompile(`sqrt\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`sqrt\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%f", math.Sqrt(val))
		}
		return m
	})

	expr = regexp.MustCompile(`abs\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`abs\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%f", math.Abs(val))
		}
		return m
	})

	expr = regexp.MustCompile(`round\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`round\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%.0f", math.Round(val))
		}
		return m
	})

	expr = regexp.MustCompile(`floor\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`floor\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%.0f", math.Floor(val))
		}
		return m
	})

	expr = regexp.MustCompile(`ceil\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`ceil\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%.0f", math.Ceil(val))
		}
		return m
	})

	expr = regexp.MustCompile(`trunc\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`trunc\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%.0f", math.Trunc(val))
		}
		return m
	})

	expr = regexp.MustCompile(`sin\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`sin\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%f", math.Sin(val))
		}
		return m
	})

	expr = regexp.MustCompile(`cos\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`cos\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%f", math.Cos(val))
		}
		return m
	})

	expr = regexp.MustCompile(`tan\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`tan\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%f", math.Tan(val))
		}
		return m
	})

	expr = regexp.MustCompile(`ln\(([^)]+)\)`).ReplaceAllStringFunc(expr, func(m string) string {
		inner := regexp.MustCompile(`ln\(([^)]+)\)`).FindStringSubmatch(m)[1]
		if val, err := strconv.ParseFloat(strings.TrimSpace(inner), 64); err == nil {
			return fmt.Sprintf("%f", math.Log(val))
		}
		return m
	})

	return expr
}

// evaluateSimpleExpression evaluates a simple mathematical expression
// This is a basic evaluator - for production, consider using a proper expression parser
func evaluateSimpleExpression(expr string) (float64, error) {
	// Remove whitespace
	expr = strings.ReplaceAll(expr, " ", "")

	// Handle power operator (^)
	expr = strings.ReplaceAll(expr, "^", "**")

	// Use Go's expression evaluation via strconv and manual parsing
	// For a more robust solution, consider using github.com/Knetic/govaluate
	// For now, we'll do basic arithmetic

	result, err := evaluateArithmetic(expr)
	if err != nil {
		return 0, err
	}
	return result, nil
}

const maxEvalIterations = 1000

// evaluateArithmetic evaluates basic arithmetic expressions
// This is simplified - handles +, -, *, /, ** (power), parentheses
func evaluateArithmetic(expr string) (float64, error) {
	// Single number (handles scientific notation like 1.5e-10)
	if v, err := parseNumber(expr); err == nil {
		return v, nil
	}

	// Handle parentheses first
	parenIter := 0
	for strings.Contains(expr, "(") {
		if parenIter++; parenIter > maxEvalIterations {
			return 0, fmt.Errorf("expression too complex (parentheses)")
		}
		start := strings.LastIndex(expr, "(")
		end := strings.Index(expr[start:], ")") + start
		if end < start {
			return 0, fmt.Errorf("unmatched parenthesis")
		}

		inner := expr[start+1 : end]
		innerResult, err := evaluateArithmetic(inner)
		if err != nil {
			return 0, err
		}

		expr = expr[:start] + fmt.Sprintf("%f", innerResult) + expr[end+1:]
	}

	// Handle power operator (**)
	powIter := 0
	for strings.Contains(expr, "**") {
		if powIter++; powIter > maxEvalIterations {
			return 0, fmt.Errorf("expression too complex (power)")
		}
		idx := strings.Index(expr, "**")
		left, right, err := splitOperator(expr, idx, 2)
		if err != nil {
			return 0, err
		}
		leftVal, err := parseNumber(left)
		if err != nil {
			return 0, err
		}
		rightVal, err := parseNumber(right)
		if err != nil {
			return 0, err
		}
		result := math.Pow(leftVal, rightVal)
		expr = fmt.Sprintf("%f", result)
	}

	// Handle multiplication and division
	mulDivIter := 0
	for strings.Contains(expr, "*") || strings.Contains(expr, "/") {
		if mulDivIter++; mulDivIter > maxEvalIterations {
			return 0, fmt.Errorf("expression too complex (mul/div)")
		}
		mulIdx := strings.Index(expr, "*")
		divIdx := strings.Index(expr, "/")

		var opIdx int
		var op string
		if mulIdx >= 0 && divIdx >= 0 {
			if mulIdx < divIdx {
				opIdx = mulIdx
				op = "*"
			} else {
				opIdx = divIdx
				op = "/"
			}
		} else if mulIdx >= 0 {
			opIdx = mulIdx
			op = "*"
		} else {
			opIdx = divIdx
			op = "/"
		}

		left, right, err := splitOperator(expr, opIdx, 1)
		if err != nil {
			return 0, err
		}
		leftVal, err := parseNumber(left)
		if err != nil {
			return 0, err
		}
		rightVal, err := parseNumber(right)
		if err != nil {
			return 0, err
		}

		var result float64
		if op == "*" {
			result = leftVal * rightVal
		} else {
			if rightVal == 0 {
				return 0, fmt.Errorf("division by zero")
			}
			result = leftVal / rightVal
		}
		expr = fmt.Sprintf("%f", result)
	}

	// Handle addition and subtraction (skip +/- that are part of scientific notation, e.g. 1e-10)
	addSubIter := 0
	for (strings.Contains(expr, "+") || strings.Contains(expr, "-")) && !strings.HasPrefix(expr, "-") {
		if addSubIter++; addSubIter > maxEvalIterations {
			return 0, fmt.Errorf("expression too complex (add/sub)")
		}
		plusIdx := indexNotInScientific(expr, "+")
		minusIdx := indexNotInScientific(expr, "-")

		var opIdx int
		var op string
		if plusIdx >= 0 && minusIdx >= 0 {
			if plusIdx < minusIdx {
				opIdx = plusIdx
				op = "+"
			} else {
				opIdx = minusIdx
				op = "-"
			}
		} else if plusIdx >= 0 {
			opIdx = plusIdx
			op = "+"
		} else if minusIdx >= 0 {
			opIdx = minusIdx
			op = "-"
		} else {
			break
		}

		left, right, err := splitOperator(expr, opIdx, 1)
		if err != nil {
			return 0, err
		}
		leftVal, err := parseNumber(left)
		if err != nil {
			return 0, err
		}
		rightVal, err := parseNumber(right)
		if err != nil {
			return 0, err
		}

		var result float64
		if op == "+" {
			result = leftVal + rightVal
		} else {
			result = leftVal - rightVal
		}
		expr = fmt.Sprintf("%f", result)
	}
	return parseNumber(expr)
}

// indexNotInScientific returns the index of the first occurrence of sep in expr
// that is not part of scientific notation (e.g. "e-" or "e+" in "1.5e-10").
func indexNotInScientific(expr, sep string) int {
	for i := 0; i <= len(expr)-len(sep); i++ {
		if expr[i:i+len(sep)] != sep {
			continue
		}
		// For "-" or "+", skip if preceded by 'e' or 'E' (scientific notation)
		if (sep == "-" || sep == "+") && i > 0 {
			if c := expr[i-1]; c == 'e' || c == 'E' {
				continue
			}
		}
		return i
	}
	return -1
}

func splitOperator(expr string, opIdx int, opLen int) (string, string, error) {
	left := strings.TrimSpace(expr[:opIdx])
	right := strings.TrimSpace(expr[opIdx+opLen:])
	return left, right, nil
}

func parseNumber(s string) (float64, error) {
	s = strings.TrimSpace(s)
	return strconv.ParseFloat(s, 64)
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
	resolved = replaceFunctions(resolved)
	result, err := evaluateSimpleExpression(resolved)
	if err != nil {
		return nil, fmt.Errorf("failed to evaluate expression: %w", err)
	}
	return result, nil
}

func resolveIntWithOverrides(value any, macro *models.Macro, overrides map[string]any) (int, error) {
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
		// Evaluate arithmetic when the value contains operators (same as ResolveInt/ResolveFloat).
		if strings.ContainsAny(resolved, "+-*/^()") {
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
