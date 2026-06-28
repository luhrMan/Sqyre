package macro

import (
	"fmt"
	"math"
	"strings"
	"sync"

	"github.com/Knetic/govaluate"
)

var (
	evalFunctionsOnce sync.Once
	evalFunctions     map[string]govaluate.ExpressionFunction
)

func expressionFunctions() map[string]govaluate.ExpressionFunction {
	evalFunctionsOnce.Do(func() {
		evalFunctions = map[string]govaluate.ExpressionFunction{
			"sqrt": func(args ...any) (any, error) {
				return math.Sqrt(asFloat(args[0])), nil
			},
			"abs": func(args ...any) (any, error) {
				return math.Abs(asFloat(args[0])), nil
			},
			"round": func(args ...any) (any, error) {
				return math.Round(asFloat(args[0])), nil
			},
			"floor": func(args ...any) (any, error) {
				return math.Floor(asFloat(args[0])), nil
			},
			"ceil": func(args ...any) (any, error) {
				return math.Ceil(asFloat(args[0])), nil
			},
			"trunc": func(args ...any) (any, error) {
				return math.Trunc(asFloat(args[0])), nil
			},
			"sin": func(args ...any) (any, error) {
				return math.Sin(asFloat(args[0])), nil
			},
			"cos": func(args ...any) (any, error) {
				return math.Cos(asFloat(args[0])), nil
			},
			"tan": func(args ...any) (any, error) {
				return math.Tan(asFloat(args[0])), nil
			},
			"ln": func(args ...any) (any, error) {
				return math.Log(asFloat(args[0])), nil
			},
		}
	})
	return evalFunctions
}

func asFloat(v any) float64 {
	switch n := v.(type) {
	case float64:
		return n
	case float32:
		return float64(n)
	case int:
		return float64(n)
	case int64:
		return float64(n)
	case int32:
		return float64(n)
	default:
		return 0
	}
}

func evaluateNumericExpression(expr string) (result float64, err error) {
	expr = strings.TrimSpace(expr)
	if expr == "" {
		return 0, fmt.Errorf("empty expression")
	}
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("invalid expression: %v", r)
		}
	}()
	e, err := govaluate.NewEvaluableExpressionWithFunctions(expr, expressionFunctions())
	if err != nil {
		return 0, err
	}
	val, err := e.Evaluate(nil)
	if err != nil {
		return 0, err
	}
	return asFloat(val), nil
}
