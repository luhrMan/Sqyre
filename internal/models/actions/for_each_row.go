package actions

import (
	"strings"

)

const (
	ForEachRowBuiltinRow      = "Row"
	ForEachRowBuiltinRowCount = "RowCount"
)

// ForEachRow iterates lines of its sources in sync and runs sub-actions per row.
// The first source drives row count; additional sources must have a line at each index.
//
// StartRow and EndRow optionally bound the iteration to a 1-based, inclusive range.
// Each may be an int literal or a "${variable}" reference. An empty/nil StartRow
// defaults to 1; an empty/nil EndRow defaults to the last row.
type ForEachRow struct {
	Sources         []ListColumn `yaml:"sources" mapstructure:"sources"`
	StartRow        any          `yaml:"startrow,omitempty" mapstructure:"startrow"`
	EndRow          any          `yaml:"endrow,omitempty" mapstructure:"endrow"`
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
}

// RowBoundIsSet reports whether a StartRow/EndRow value is meaningfully set
// (non-nil and, for strings, non-blank).
func RowBoundIsSet(v any) bool {
	switch t := v.(type) {
	case nil:
		return false
	case string:
		return strings.TrimSpace(t) != ""
	default:
		return true
	}
}

func NewForEachRow(name string, sources []ListColumn, subActions []ActionInterface) *ForEachRow {
	if sources == nil {
		sources = []ListColumn{}
	}
	return &ForEachRow{
		AdvancedAction: newAdvancedAction(name, "foreachrow", subActions),
		Sources:        sources,
	}
}

func (a *ForEachRow) Reset() {
	for i := range a.Sources {
		a.Sources[i].Reset()
	}
}

func (a *ForEachRow) String() string {
	return stringifyParams(a.Params())
}

func (a *ForEachRow) Params() []Param {
	params := []Param{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Sources", len(a.Sources)),
	}
	if RowBoundIsSet(a.StartRow) {
		params = append(params, newExtraParam("Start Row", a.StartRow))
	}
	if RowBoundIsSet(a.EndRow) {
		params = append(params, newExtraParam("End Row", a.EndRow))
	}
	return params
}

func (a *ForEachRow) VariableBindings() []VariableBinding {
	var out []VariableBinding
	for _, s := range a.Sources {
		if s.OutputVar != "" {
			out = append(out, VariableBinding{Name: s.OutputVar, Role: "output", Conditional: true})
		}
	}
	return out
}
