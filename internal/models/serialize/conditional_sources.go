package serialize

import "Sqyre/internal/models/actions"

func clauseFromMap(m map[string]any) actions.ConditionClause {
	op := stringFromMap(m, "operator")
	if op == "" {
		op = actions.OpEquals
	}
	return actions.ConditionClause{
		Left:     operandFromMap(m, "left"),
		Operator: op,
		Right:    operandFromMap(m, "right"),
	}
}

// clausesListFromMap decodes a clauses array, skipping entries that are not mappings.
func clausesListFromMap(v any) []actions.ConditionClause {
	if v == nil {
		return nil
	}
	list, err := anySlice(v)
	if err != nil {
		return nil
	}
	out := make([]actions.ConditionClause, 0, len(list))
	for _, item := range list {
		m, ok := item.(map[string]any)
		if !ok {
			continue
		}
		out = append(out, clauseFromMap(m))
	}
	return out
}

// clausesFromMap decodes conditional clauses from the clauses array.
// When nothing usable is present, returns a single empty default clause.
func clausesFromMap(rawMap map[string]any) []actions.ConditionClause {
	if cols := clausesListFromMap(rawMap["clauses"]); len(cols) > 0 {
		return cols
	}
	return []actions.ConditionClause{{Left: "", Operator: actions.OpEquals, Right: ""}}
}
