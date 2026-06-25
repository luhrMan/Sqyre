package serialize

import (
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

func listColumnFromMap(m map[string]any) actions.ListColumn {
	return actions.ListColumn{
		Source:         stringFromMap(m, "source"),
		OutputVar:      stringFromMap(m, "outputvar"),
		IsFile:         boolFromMap(m, "isfile"),
		SkipBlankLines: boolFromMap(m, "skipblanklines"),
	}
}

// sourcesListFromMap decodes a sources array, skipping entries that are not mappings.
// Returns nil when v is nil or not a list.
func sourcesListFromMap(v any) []actions.ListColumn {
	if v == nil {
		return nil
	}
	list, err := anySlice(v)
	if err != nil {
		return nil
	}
	out := make([]actions.ListColumn, 0, len(list))
	for _, item := range list {
		m, ok := item.(map[string]any)
		if !ok {
			continue
		}
		out = append(out, listColumnFromMap(m))
	}
	return out
}

// forEachRowSourcesFromMap loads For Each Row sources from the sources array.
// When nothing usable is present, returns an empty slice.
func forEachRowSourcesFromMap(rawMap map[string]any) []actions.ListColumn {
	cols := sourcesListFromMap(rawMap["sources"])
	if cols == nil {
		return []actions.ListColumn{}
	}
	return cols
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
