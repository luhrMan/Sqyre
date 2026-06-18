package serialize

import (
	"fmt"

	"Sqyre/internal/models/actions"
)

func boolFromMap(m map[string]any, key string) bool {
	v, ok := m[key]
	if !ok || v == nil {
		return false
	}
	b, ok := v.(bool)
	return ok && b
}

func sourcesFromMap(v any) ([]actions.ListColumn, error) {
	if v == nil {
		return []actions.ListColumn{}, nil
	}
	var raw []any
	switch t := v.(type) {
	case []any:
		raw = t
	case []map[string]any:
		raw = make([]any, len(t))
		for i, m := range t {
			raw[i] = m
		}
	default:
		return nil, fmt.Errorf("sources: expected array, got %T", v)
	}
	out := make([]actions.ListColumn, 0, len(raw))
	for i, item := range raw {
		m, ok := item.(map[string]any)
		if !ok {
			return nil, fmt.Errorf("sources[%d]: expected mapping, got %T", i, item)
		}
		src, err := expectString(m, "source")
		if err != nil {
			return nil, fmt.Errorf("sources[%d]: %w", i, err)
		}
		outVar, err := expectString(m, "outputvar")
		if err != nil {
			return nil, fmt.Errorf("sources[%d]: %w", i, err)
		}
		out = append(out, actions.ListColumn{
			Source:         src,
			OutputVar:      outVar,
			IsFile:         boolFromMap(m, "isfile"),
			SkipBlankLines: boolFromMap(m, "skipblanklines"),
		})
	}
	return out, nil
}

func listColumnsToMaps(cols []actions.ListColumn) []map[string]any {
	out := make([]map[string]any, len(cols))
	for i, c := range cols {
		m := map[string]any{
			"source":    c.Source,
			"outputvar": c.OutputVar,
		}
		if c.IsFile {
			m["isfile"] = true
		}
		if c.SkipBlankLines {
			m["skipblanklines"] = true
		}
		out[i] = m
	}
	return out
}
