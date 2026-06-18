package actions

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// Comparison operators supported by the Conditional action.
const (
	OpEquals     = "=="
	OpNotEquals  = "!="
	OpLess       = "<"
	OpLessEqual  = "<="
	OpGreater    = ">"
	OpGreaterEq  = ">="
	OpContains   = "contains"
	OpStartsWith = "starts with"
	OpEndsWith   = "ends with"
	OpIsSet      = "is set"
	OpIsEmpty    = "is empty"
)

// ConditionalOperators lists every operator in display order for UI selectors.
var ConditionalOperators = []string{
	OpEquals,
	OpNotEquals,
	OpLess,
	OpLessEqual,
	OpGreater,
	OpGreaterEq,
	OpContains,
	OpStartsWith,
	OpEndsWith,
	OpIsSet,
	OpIsEmpty,
}

// OperatorIsUnary reports whether an operator only uses the left operand
// (the right operand is ignored for these).
func OperatorIsUnary(op string) bool {
	return op == OpIsSet || op == OpIsEmpty
}

// Conditional is an advanced action that compares two operands and, when the
// comparison is true, runs its sub-actions (the branch). When false, execution
// continues past the branch without running the sub-actions.
//
// Left and Right may be int (literal) or string (literal or variable reference
// e.g. "${score}").
type Conditional struct {
	Left            any
	Operator        string
	Right           any
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

func NewConditional(left any, operator string, right any, name string, subActions []ActionInterface) *Conditional {
	if operator == "" {
		operator = OpEquals
	}
	return &Conditional{
		AdvancedAction: newAdvancedAction(name, "conditional", subActions),
		Left:           left,
		Operator:       operator,
		Right:          right,
	}
}

func (a *Conditional) String() string {
	return stringifyParams(a.parameters())
}

func (a *Conditional) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Conditional) parameters() []actionParam {
	params := []actionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("If", a.Left),
		newParam("Op", a.Operator),
	}
	if !OperatorIsUnary(a.Operator) {
		params = append(params, newParam("Value", a.Right))
	}
	return params
}

func (a *Conditional) Icon() fyne.Resource {
	return theme.QuestionIcon()
}
