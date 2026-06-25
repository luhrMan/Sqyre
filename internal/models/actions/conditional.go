package actions

import (
	"fmt"
	"strings"

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

// Match modes for combining multiple condition clauses.
const (
	MatchAll = "all" // AND — every clause must be true
	MatchAny = "any" // OR — at least one clause must be true
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

// ConditionalMatchModes lists match modes in display order for UI selectors.
var ConditionalMatchModes = []string{MatchAll, MatchAny}

// OperatorIsUnary reports whether an operator only uses the left operand
// (the right operand is ignored for these).
func OperatorIsUnary(op string) bool {
	return op == OpIsSet || op == OpIsEmpty
}

// ConditionClause is one comparison within a Conditional action.
// Left and Right may be int (literal) or string (literal or variable reference
// e.g. "${score}").
type ConditionClause struct {
	Left     any
	Operator string
	Right    any
}

// Summary returns a compact human-readable form of the clause.
func (c ConditionClause) Summary() string {
	if OperatorIsUnary(c.Operator) {
		return fmt.Sprintf("%v %s", c.Left, c.Operator)
	}
	return fmt.Sprintf("%v %s %v", c.Left, c.Operator, c.Right)
}

// Conditional is an advanced action that evaluates one or more comparisons and,
// when the combined result is true, runs its sub-actions (the branch). When false,
// execution continues past the branch without running the sub-actions.
type Conditional struct {
	Clauses []ConditionClause
	Match   string // MatchAll (AND) or MatchAny (OR); empty defaults to MatchAll
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

// EffectiveMatch returns MatchAny when set, otherwise MatchAll.
func (a *Conditional) EffectiveMatch() string {
	if a.Match == MatchAny {
		return MatchAny
	}
	return MatchAll
}

// MatchLabel returns a display label for the current match mode.
func (a *Conditional) MatchLabel() string {
	if a.EffectiveMatch() == MatchAny {
		return "any (OR)"
	}
	return "all (AND)"
}

func NewConditional(clauses []ConditionClause, match string, name string, subActions []ActionInterface) *Conditional {
	if len(clauses) == 0 {
		clauses = []ConditionClause{{Left: "", Operator: OpEquals, Right: ""}}
	}
	for i := range clauses {
		if clauses[i].Operator == "" {
			clauses[i].Operator = OpEquals
		}
	}
	return &Conditional{
		AdvancedAction: newAdvancedAction(name, "conditional", subActions),
		Clauses:        clauses,
		Match:          match,
	}
}

func (a *Conditional) conditionSummary() string {
	sep := " AND "
	if a.EffectiveMatch() == MatchAny {
		sep = " OR "
	}
	parts := make([]string, len(a.Clauses))
	for i, c := range a.Clauses {
		parts[i] = c.Summary()
	}
	return strings.Join(parts, sep)
}

func (a *Conditional) String() string {
	return stringifyParams(a.parameters())
}

func (a *Conditional) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Conditional) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Match", a.MatchLabel()),
		newParam("If", a.conditionSummary()),
	}
}

func (a *Conditional) Icon() fyne.Resource {
	return theme.QuestionIcon()
}
