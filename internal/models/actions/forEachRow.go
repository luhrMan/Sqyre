package actions

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

const (
	ForEachRowBuiltinRow      = "Row"
	ForEachRowBuiltinRowCount = "RowCount"
)

// ForEachRow iterates every line of its sources in sync and runs sub-actions per row.
// The first source drives row count; additional sources must have a line at each index.
type ForEachRow struct {
	Sources         []ListColumn `yaml:"sources" mapstructure:"sources"`
	*AdvancedAction `yaml:",inline" mapstructure:",squash"`
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
	return stringifyParams(a.parameters())
}

func (a *ForEachRow) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *ForEachRow) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Sources", len(a.Sources)),
	}
}

func (a *ForEachRow) Icon() fyne.Resource {
	return theme.ListIcon()
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
