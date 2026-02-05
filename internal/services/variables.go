package services

import (
	"Squire/internal/models"
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

// ResolveVariables resolves variable references in a string
// Supports ${VarName} and {VarName} syntax
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
