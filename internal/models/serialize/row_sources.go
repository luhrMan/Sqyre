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

// forEachRowSourcesFromMap loads For Each Row sources from the current serialized
// shape (sources array) or legacy flat fields (DataList / early foreachrow).
// When nothing usable is present, returns an empty slice.
func forEachRowSourcesFromMap(rawMap map[string]any) []actions.ListColumn {
	if cols := sourcesListFromMap(rawMap["sources"]); len(cols) > 0 {
		return cols
	}
	if hasLegacyForEachRowFields(rawMap) {
		return []actions.ListColumn{listColumnFromMap(rawMap)}
	}
	return []actions.ListColumn{}
}

func hasLegacyForEachRowFields(rawMap map[string]any) bool {
	for _, key := range []string{"source", "outputvar", "isfile", "skipblanklines"} {
		if v, ok := rawMap[key]; ok && v != nil {
			if s, ok := v.(string); ok && s == "" {
				continue
			}
			return true
		}
	}
	return false
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
