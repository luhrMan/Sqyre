// Package varref is the single source of truth for the macro variable-reference
// grammar: ${name} and {name} (the latter only when not preceded by '$').
//
// It holds the low-level parsing/scanning primitives (segmenting display text,
// extracting referenced names, matching/renaming a specific name). Higher-level
// concerns — value substitution, expression evaluation, usage collection — live
// in internal/macro and internal/models and build on these primitives.
package varref

import (
	"regexp"
	"strings"
)

// Segment is one plain-text or variable-reference segment of a string.
type Segment struct {
	Text  string
	IsRef bool
	Name  string
}

var (
	// DollarPattern matches ${name}; name is any run of non-'}' characters.
	// Exported so the substitution engine (internal/macro) sources the grammar
	// from here rather than re-declaring it.
	DollarPattern = regexp.MustCompile(`\$\{([^}]+)\}`)
	// BracePattern matches {name}. The "{name} only when not preceded by '$'"
	// distinction is handled by callers (Names dedups, so the overlap is harmless).
	BracePattern = regexp.MustCompile(`\{([^}]+)\}`)
)

// Contains reports whether text contains a ${name} or {name} reference.
func Contains(text string) bool {
	if text == "" {
		return false
	}
	if DollarPattern.MatchString(text) {
		return true
	}
	return findBraceRefs(text) != nil
}

// Names returns the distinct raw variable names referenced in text (untrimmed),
// from both ${name} and {name} forms.
func Names(text string) []string {
	names := make(map[string]bool)
	for _, m := range DollarPattern.FindAllStringSubmatch(text, -1) {
		if len(m) > 1 {
			names[m[1]] = true
		}
	}
	for _, m := range BracePattern.FindAllStringSubmatch(text, -1) {
		if len(m) > 1 {
			names[m[1]] = true
		}
	}
	out := make([]string, 0, len(names))
	for name := range names {
		out = append(out, name)
	}
	return out
}

// Segments splits text into plain and reference segments in document order.
// References use ${name} or {name} (not the inner brace of ${...}).
func Segments(text string) []Segment {
	if text == "" {
		return nil
	}
	var matches []match
	for _, loc := range DollarPattern.FindAllStringSubmatchIndex(text, -1) {
		matches = append(matches, match{loc[0], loc[1], text[loc[2]:loc[3]]})
	}
	matches = append(matches, findBraceRefs(text)...)
	if len(matches) == 0 {
		return []Segment{{Text: text}}
	}
	sortMatches(matches)
	filtered := dropOverlapping(matches)
	segs := make([]Segment, 0, len(filtered)*2+1)
	last := 0
	for _, m := range filtered {
		if m.start > last {
			segs = append(segs, Segment{Text: text[last:m.start]})
		}
		segs = append(segs, Segment{
			Text:  text[m.start:m.end],
			IsRef: true,
			Name:  m.name,
		})
		last = m.end
	}
	if last < len(text) {
		segs = append(segs, Segment{Text: text[last:]})
	}
	return segs
}

// References reports whether text references the variable name (case-insensitive,
// tolerating whitespace inside the braces), in either ${name} or {name} form.
func References(text, name string) bool {
	if text == "" || name == "" {
		return false
	}
	quoted := regexp.QuoteMeta(strings.TrimSpace(name))
	dollar := regexp.MustCompile(`(?i)\$\{\s*` + quoted + `\s*\}`)
	if dollar.MatchString(text) {
		return true
	}
	brace := regexp.MustCompile(`(?i)(^|[^$])\{\s*` + quoted + `\s*\}`)
	return brace.MatchString(text)
}

// Rename replaces ${old} and {old} references (case-insensitive, tolerating
// surrounding spaces) with newName, preserving the brace style.
func Rename(s, oldName, newName string) string {
	if s == "" {
		return s
	}
	quoted := regexp.QuoteMeta(oldName)
	// "$$" emits a literal "$" in regexp replacement strings.
	dollar := regexp.MustCompile(`(?i)\$\{\s*` + quoted + `\s*\}`)
	s = dollar.ReplaceAllString(s, "$${"+newName+"}")
	brace := regexp.MustCompile(`(?i)(^|[^$])\{\s*` + quoted + `\s*\}`)
	s = brace.ReplaceAllString(s, "${1}{"+newName+"}")
	return s
}

type match struct {
	start, end int
	name       string
}

func findBraceRefs(text string) []match {
	var out []match
	for i := 0; i < len(text); i++ {
		if text[i] != '{' {
			continue
		}
		if i > 0 && text[i-1] == '$' {
			continue
		}
		end := -1
		for j := i + 1; j < len(text); j++ {
			if text[j] == '}' {
				end = j
				break
			}
		}
		if end < 0 {
			continue
		}
		name := text[i+1 : end]
		if name == "" {
			continue
		}
		out = append(out, match{i, end + 1, name})
		i = end
	}
	return out
}

func sortMatches(matches []match) {
	for i := range matches {
		for j := i + 1; j < len(matches); j++ {
			if matches[j].start < matches[i].start {
				matches[i], matches[j] = matches[j], matches[i]
			}
		}
	}
}

func dropOverlapping(matches []match) []match {
	filtered := matches[:0]
	for _, m := range matches {
		overlap := false
		for _, prev := range filtered {
			if m.start >= prev.start && m.end <= prev.end {
				overlap = true
				break
			}
		}
		if !overlap {
			filtered = append(filtered, m)
		}
	}
	return filtered
}